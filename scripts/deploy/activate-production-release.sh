#!/usr/bin/env bash
# Activate artifacts uploaded by the main-branch production workflow.
set -Eeuo pipefail

release_id=${1:-}
deploy_path=${2:-}
api_port=${3:-}
web_port=${4:-}

if [[ ! "$release_id" =~ ^[0-9a-f]{7,40}-[0-9]+$ ]]; then
  echo "Invalid release ID" >&2
  exit 2
fi
if [[ ! "$deploy_path" =~ ^/[A-Za-z0-9._/-]+$ ]]; then
  echo "DEPLOY_PATH must be a safe absolute path" >&2
  exit 2
fi
if [[ ! "$api_port" =~ ^[0-9]{1,5}$ ]] || ((api_port < 1 || api_port > 65535)); then
  echo "Invalid API port" >&2
  exit 2
fi
if [[ ! "$web_port" =~ ^[0-9]{1,5}$ ]] || ((web_port < 1 || web_port > 65535)); then
  echo "Invalid Web port" >&2
  exit 2
fi

staging_dir="/tmp/lumiforum-${release_id}"
api_root="${deploy_path}/api"
web_root="${deploy_path}/web"
api_release_dir="${api_root}/releases/${release_id}"
web_release_dir="${web_root}/releases/${release_id}"
web_candidate_dir="${web_root}/releases/.${release_id}.candidate"
api_binary="${api_root}/lumiforum-api"
migrate_binary="${api_root}/migrate"
web_current="${web_root}/current"

cleanup() {
  rm -rf -- "$staging_dir" "$web_candidate_dir"
  rm -f -- "${api_binary}.next" "${migrate_binary}.next" "${web_root}/current.next"
}
trap cleanup EXIT

cd "$staging_dir"
sha256sum --check SHA256SUMS

test -f lumiforum-api
test -f migrate
test -f lumiforum-web.tar.gz
test -f BUILD-INFO
test -f "$api_root/.env"
test -f "$web_root/.env"

if [[ -e "$api_release_dir" || -e "$web_release_dir" ]]; then
  echo "Release already exists: $release_id" >&2
  exit 1
fi

install -d -m 755 "$api_root/releases" "$web_root/releases"
install -d -m 755 "$api_release_dir" "$web_candidate_dir"
install -m 755 lumiforum-api "$api_release_dir/lumiforum-api"
install -m 755 migrate "$api_release_dir/migrate"
install -m 644 BUILD-INFO "$api_release_dir/BUILD-INFO"
tar -xzf lumiforum-web.tar.gz -C "$web_candidate_dir"
install -m 644 BUILD-INFO "$web_candidate_dir/BUILD-INFO"

test -x "$api_release_dir/lumiforum-api"
test -x "$api_release_dir/migrate"
test -f "$web_candidate_dir/apps/web/server.js"
mv "$web_candidate_dir" "$web_release_dir"

# Migrations are embedded in the candidate binary and run before activation.
systemd-run --user --wait --pipe --collect \
  --property="EnvironmentFile=$api_root/.env" \
  --property="WorkingDirectory=$api_release_dir" \
  "$api_release_dir/migrate"

backup_id="rollback-${release_id}"
api_backup_dir="${api_root}/releases/${backup_id}"
web_backup_target=""
had_api=false
had_migrate=false

# Preserve the API files currently used by the fixed production service path.
install -d -m 755 "$api_backup_dir"
if [[ -e "$api_binary" ]]; then
  cp -aL "$api_binary" "$api_backup_dir/lumiforum-api"
  had_api=true
fi
if [[ -e "$migrate_binary" ]]; then
  cp -aL "$migrate_binary" "$api_backup_dir/migrate"
  had_migrate=true
fi

# Existing installations may have web/current as a real directory. Preserve it
# before converting current into an atomic release symlink.
if [[ -L "$web_current" ]]; then
  web_backup_target=$(readlink "$web_current")
elif [[ -d "$web_current" ]]; then
  web_backup_target="releases/${backup_id}"
  mv "$web_current" "${web_root}/${web_backup_target}"
elif [[ -e "$web_current" ]]; then
  echo "Expected web/current to be a directory, symlink, or absent path" >&2
  exit 1
fi

activate_api() {
  local source_dir=$1
  install -m 755 "$source_dir/lumiforum-api" "${api_binary}.next" || return 1
  install -m 755 "$source_dir/migrate" "${migrate_binary}.next" || return 1
  mv -Tf "${api_binary}.next" "$api_binary" || return 1
  mv -Tf "${migrate_binary}.next" "$migrate_binary" || return 1
}

activate_web() {
  local target=$1
  ln -sfn "$target" "${web_root}/current.next" || return 1
  mv -Tf "${web_root}/current.next" "$web_current" || return 1
}

wait_for_url() {
  local url=$1
  local attempts=${2:-30}
  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if curl --fail --silent --show-error --max-time 5 --output /dev/null "$url"; then
      return 0
    fi
    sleep 2
  done
  return 1
}

rollback() {
  echo "Deployment failed; restoring previous application release" >&2
  if [[ "$had_api" == true ]]; then
    install -m 755 "$api_backup_dir/lumiforum-api" "${api_binary}.next" || true
    mv -Tf "${api_binary}.next" "$api_binary" || true
  else
    rm -f "$api_binary"
  fi
  if [[ "$had_migrate" == true ]]; then
    install -m 755 "$api_backup_dir/migrate" "${migrate_binary}.next" || true
    mv -Tf "${migrate_binary}.next" "$migrate_binary" || true
  else
    rm -f "$migrate_binary"
  fi
  if [[ -n "$web_backup_target" ]]; then
    activate_web "$web_backup_target" || true
  else
    rm -f "$web_current"
  fi
  systemctl --user restart lumiforum-api lumiforum-web || true
  systemctl --user show lumiforum-api lumiforum-web \
    --property=Id --property=ActiveState --property=SubState --property=Result || true
}

if ! activate_api "$api_release_dir"; then
  rollback
  exit 1
fi
if ! activate_web "releases/$release_id"; then
  rollback
  exit 1
fi

if ! systemctl --user restart lumiforum-api lumiforum-web; then
  rollback
  exit 1
fi
if ! wait_for_url "http://127.0.0.1:${api_port}/ready"; then
  rollback
  exit 1
fi
if ! wait_for_url "http://127.0.0.1:${web_port}/"; then
  rollback
  exit 1
fi

systemctl --user show lumiforum-api lumiforum-web \
  --property=Id --property=ActiveState --property=SubState --property=Result
echo "Production release activated: $release_id"

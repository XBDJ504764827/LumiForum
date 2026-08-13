/**
 * LumiForum — 本地开发用 Steam Auth 中继（开发环境专用）
 * ======================================================
 * 生产环境使用 Cloudflare Worker（scripts/deploy/steam-auth-worker.js）。
 * 开发环境不应把玩家重定向到生产地址，本脚本在本地起一个与 Worker
 * 相同契约的中继服务，用于本地联调。
 *
 * 两种模式（环境变量 STEAM_DEV_RELAY_MODE）：
 *   - mock（默认）：不访问真实 Steam。跳转到本地的模拟登录页，
 *     输入任意 17 位 SteamID64 即可完成登录，适合快速联调
 *     （注册/绑定/联系方式补全/管理员追溯等流程）。
 *   - real：走真实 Steam OpenID 登录（realm = 本中继的 origin，
 *     需本机可访问 steamcommunity.com）。若配置了 STEAM_API_KEY，
 *     则 /verify 与 /profile 会返回真实玩家资料。
 *
 * 端点（与 Worker 保持一致）：
 *   GET /                             健康检查
 *   GET /login?mode=&state=&return_to=   发起登录 → 302
 *   GET /callback                       （real 模式）Steam OpenID 回跳
 *   GET /verify?token=                  一次性 token 换 SteamID（用后即焚）
 *   GET /profile?steamid=               按 SteamID 返回资料
 *
 * 启动：node scripts/dev-steam-relay.mjs
 * 环境变量：
 *   STEAM_DEV_RELAY_HOST  监听地址，默认 0.0.0.0
 *   STEAM_DEV_RELAY_PORT  监听端口，默认 8787
 *   STEAM_DEV_RELAY_MODE  mock | real，默认 mock
 *   STEAM_API_KEY         （real 模式可选）Steam Web API Key
 */
import { createServer } from "node:http";
import { randomBytes } from "node:crypto";

const HOST = process.env.STEAM_DEV_RELAY_HOST || "0.0.0.0";
const PORT = Number(process.env.STEAM_DEV_RELAY_PORT || 8787);
const MODE = process.env.STEAM_DEV_RELAY_MODE || "mock";
const STEAM_API_KEY = process.env.STEAM_API_KEY || "";
const TOKEN_TTL_MS = 5 * 60 * 1_000;

/** 一次性 token：token -> 记录，取用后删除（单次消费，防重放）。 */
const tokens = new Map();
/** mock 模式下按 SteamID 记忆玩家资料（供 /profile 同步使用）。 */
const mockProfiles = new Map();

const json = (res, body, status = 200) => {
  res.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(body));
};

const redirect = (res, target, status = 302) => {
  res.writeHead(status, { Location: target });
  res.end();
};

const originOf = (req) => {
  const host = req.headers.host || `127.0.0.1:${PORT}`;
  return `http://${host}`;
};

/** 302 跳转到调用方回调地址并附带查询参数（与 Worker redirectWith 一致）。 */
function redirectWith(returnTo, query) {
  let target;
  try {
    target = new URL(returnTo);
  } catch {
    return null;
  }
  if (target.protocol !== "http:" && target.protocol !== "https:") return null;
  for (const [key, value] of Object.entries(query)) {
    if (value) target.searchParams.set(key, value);
  }
  return target.toString();
}

function mintToken(record) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const token = randomBytes(16).toString("hex");
    if (tokens.has(token)) continue;
    tokens.set(token, { ...record, expiresAt: Date.now() + TOKEN_TTL_MS });
    return token;
  }
  throw new Error("failed to allocate one-time token");
}

function consumeToken(token) {
  const record = tokens.get(token);
  if (!record) return null;
  tokens.delete(token); // 单次消费
  if (record.expiresAt <= Date.now()) return null;
  return record;
}

function validateSteamId(steamid) {
  return /^\d{17}$/.test(String(steamid));
}

const html = (body) => `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>本地 Steam 模拟登录（开发环境）</title>
<style>
  body { font-family: system-ui, sans-serif; background: #1b2838; color: #c7d5e0;
         display: flex; min-height: 100vh; align-items: center; justify-content: center; margin: 0; }
  .card { background: #16202d; border: 1px solid #2a475e; border-radius: 8px;
          padding: 28px 32px; width: 100%; max-width: 380px; }
  h1 { font-size: 18px; margin: 0 0 4px; }
  .hint { font-size: 12px; color: #8f98a0; margin: 0 0 18px; }
  label { display: block; font-size: 12px; margin: 12px 0 4px; }
  input { width: 100%; box-sizing: border-box; background: #101822; color: #c7d5e0;
          border: 1px solid #2a475e; border-radius: 4px; padding: 8px 10px; font-size: 14px; }
  button { width: 100%; margin-top: 18px; background: #66c0f4; border: 0; border-radius: 4px;
           padding: 10px; color: #101822; font-size: 14px; font-weight: 600; cursor: pointer; }
  .cancel { background: transparent; color: #8f98a0; border: 1px solid #2a475e;
            font-weight: 400; margin-top: 8px; }
  .badge { display: inline-block; background: #417a9b; color: #fff; font-size: 11px;
           border-radius: 3px; padding: 1px 6px; vertical-align: middle; }
</style>
</head>
<body>${body}</body>
</html>`;

/** mock 模式：本地模拟 Steam 登录页。 */
function mockLoginPage(req, res) {
  const url = new URL(req.url, originOf(req));
  const state = url.searchParams.get("state") || "";
  const returnTo = url.searchParams.get("return_to") || "";
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(
    html(`
<div class="card">
  <h1>Steam 模拟登录 <span class="badge">DEV ONLY</span></h1>
  <p class="hint">本地开发中继（mock 模式），不会访问真实 Steam。输入任意 17 位数字
  SteamID64 即可模拟一次 Steam 登录，用于联调论坛的登录 / 联系方式绑定流程。</p>
  <form method="POST" action="/mock-authorize">
    <input type="hidden" name="state" value="${escapeHtml(state)}" />
    <input type="hidden" name="return_to" value="${escapeHtml(returnTo)}" />
    <label for="steamid">SteamID64（17 位数字）</label>
    <input id="steamid" name="steamid" placeholder="76561198000000000" pattern="\\d{17}"
           maxlength="17" required autofocus />
    <label for="persona_name">昵称（可选，用于模拟 Steam 资料）</label>
    <input id="persona_name" name="persona_name" placeholder="例如：Lumi_Dev" maxlength="64" />
    <button type="submit">登录</button>
    <button class="cancel" type="submit" formaction="/mock-cancel">取消授权</button>
  </form>
</div>`),
  );
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

/** mock 模式：模拟“Steam 已完成验证”，签发一次性 token 并跳回调用方。 */
function mockAuthorize(req, res) {
  let body = "";
  req.on("data", (chunk) => {
    body += chunk;
    if (body.length > 16 * 1024) req.destroy();
  });
  req.on("end", () => {
    const params = new URLSearchParams(body);
    const state = params.get("state") || "";
    const returnTo = params.get("return_to") || "";
    const target = redirectWith(returnTo, { state, error: "steam_auth_failed" });
    if (!target) {
      json(res, { success: false, error: "invalid return_to" }, 400);
      return;
    }
    const steamid = (params.get("steamid") || "").trim();
    if (!validateSteamId(steamid)) {
      redirect(res, target);
      return;
    }
    const personaName = (params.get("persona_name") || "").trim().slice(0, 64);
    if (personaName) {
      mockProfiles.set(steamid, {
        steamid,
        persona_name: personaName,
        avatar: null,
        avatar_medium: null,
        avatar_full: null,
        profile_url: null,
        country_code: null,
      });
    }
    const token = mintToken({
      steamid,
      persona_name: personaName || null,
      avatar: null,
      avatar_medium: null,
      avatar_full: null,
      profile_url: null,
      country_code: null,
    });
    const done = redirectWith(returnTo, { token, state });
    if (!done) {
      json(res, { success: false, error: "invalid return_to" }, 400);
      return;
    }
    redirect(res, done);
  });
}

/** mock 模式：模拟玩家在 Steam 侧取消授权。 */
function mockCancel(req, res) {
  let body = "";
  req.on("data", (chunk) => {
    body += chunk;
    if (body.length > 16 * 1024) req.destroy();
  });
  req.on("end", () => {
    const params = new URLSearchParams(body);
    const target = redirectWith(params.get("return_to") || "", {
      state: params.get("state") || "",
      error: "steam_access_denied",
    });
    if (!target) {
      json(res, { success: false, error: "invalid return_to" }, 400);
      return;
    }
    redirect(res, target);
  });
}

/** real 模式：302 到真实 Steam OpenID 登录页（realm = 本中继 origin）。 */
function realLogin(req, res) {
  const origin = originOf(req);
  const url = new URL(req.url, origin);
  const state = url.searchParams.get("state") || "";
  const returnTo = url.searchParams.get("return_to") || "";

  const workerCallback = new URL(`${origin}/callback`);
  if (state) workerCallback.searchParams.set("state", state);
  workerCallback.searchParams.set("return_to", returnTo);

  const steam = new URL("https://steamcommunity.com/openid/login");
  const params = {
    "openid.ns": "http://specs.openid.net/auth/2.0",
    "openid.mode": "checkid_setup",
    "openid.return_to": workerCallback.toString(),
    "openid.realm": origin,
    "openid.identity": "http://specs.openid.net/auth/2.0/identifier_select",
    "openid.claimed_id": "http://specs.openid.net/auth/2.0/identifier_select",
  };
  for (const [key, value] of Object.entries(params)) {
    steam.searchParams.set(key, value);
  }
  redirect(res, steam.toString(), 302);
}

/** real 模式：Steam OpenID 回跳，验证断言后签发一次性 token。 */
async function realCallback(req, res) {
  const origin = originOf(req);
  const url = new URL(req.url, origin);
  const params = new URLSearchParams(url.search);
  const returnTo = params.get("return_to") || "";
  const state = params.get("state") || "";
  const fail = redirectWith(returnTo, { error: "steam_auth_failed", state });

  if (params.get("openid.mode") === "cancel") {
    const target = redirectWith(returnTo, { error: "steam_access_denied", state });
    if (target) redirect(res, target);
    else json(res, { success: false, error: "invalid return_to" }, 400);
    return;
  }
  if (params.get("openid.mode") !== "id_res") {
    redirect(res, fail || "about:blank");
    return;
  }

  params.set("openid.mode", "check_authentication");
  let verified = false;
  try {
    const response = await fetch("https://steamcommunity.com/openid/login", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: params.toString(),
    });
    verified = (await response.text()).includes("is_valid:true");
  } catch (error) {
    console.error("[dev-steam-relay] check_authentication failed:", error.message);
  }
  if (!verified) {
    redirect(res, fail || "about:blank");
    return;
  }

  const claimed = params.get("openid.claimed_id") || "";
  const steamid = claimed.split("/").pop() || "";
  if (!validateSteamId(steamid)) {
    redirect(res, fail || "about:blank");
    return;
  }

  const profile = STEAM_API_KEY ? await fetchSteamProfile(steamid).catch(() => null) : null;
  const token = mintToken({ steamid, ...(profile || {}) });
  const target = redirectWith(returnTo, { token, state });
  if (target) redirect(res, target);
  else json(res, { success: false, error: "invalid return_to" }, 400);
}

/** 一次性 token 换 SteamID（论坛后端调用，单次有效）。 */
function verify(req, res) {
  const url = new URL(req.url, originOf(req));
  const token = url.searchParams.get("token") || "";
  const record = token ? consumeToken(token) : null;
  if (!record) {
    json(res, { success: false, error: "invalid or expired token" }, 401);
    return;
  }
  json(res, { success: true, ...record });
}

/** 按 SteamID 返回资料（mock 模式回放本中继记忆的资料，否则给默认昵称）。 */
async function profile(req, res) {
  const url = new URL(req.url, originOf(req));
  const steamid = url.searchParams.get("steamid") || "";
  if (!validateSteamId(steamid)) {
    json(res, { success: false, error: "invalid steamid" }, 400);
    return;
  }
  const remembered = mockProfiles.get(steamid);
  if (remembered) {
    json(res, { success: true, ...remembered });
    return;
  }
  if (STEAM_API_KEY) {
    const real = await fetchSteamProfile(steamid).catch(() => null);
    if (real) {
      json(res, { success: true, steamid, ...real });
      return;
    }
  }
  json(res, {
    success: true,
    steamid,
    persona_name: `steam_${steamid}`,
    avatar: null,
    avatar_medium: null,
    avatar_full: null,
    profile_url: null,
    country_code: null,
  });
}

/** real 模式：通过 Steam Web API 拉取玩家资料。 */
async function fetchSteamProfile(steamid) {
  const response = await fetch(
    `https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key=${encodeURIComponent(STEAM_API_KEY)}&steamids=${steamid}`,
  );
  if (!response.ok) return null;
  const data = await response.json();
  const player = data?.response?.players?.[0];
  if (!player || player.steamid !== steamid) return null;
  return {
    persona_name: player.personaname || null,
    avatar: player.avatar || null,
    avatar_medium: player.avatarmedium || null,
    avatar_full: player.avatarfull || null,
    profile_url: player.profileurl || null,
    country_code: player.loccountrycode || null,
  };
}

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, originOf(req));
    if (url.pathname === "/") {
      json(res, {
        service: "LumiForum dev Steam auth relay",
        mode: MODE,
        status: "running",
      });
      return;
    }
    if (url.pathname === "/login") {
      if (MODE === "real") realLogin(req, res);
      else {
        // mock：跳到本地模拟登录页，保留 state 与 return_to
        const target = new URL(`${originOf(req)}/mock-login`);
        for (const key of ["mode", "state", "return_to"]) {
          const value = url.searchParams.get(key);
          if (value) target.searchParams.set(key, value);
        }
        redirect(res, target.toString());
      }
      return;
    }
    if (url.pathname === "/mock-login") {
      mockLoginPage(req, res);
      return;
    }
    if (url.pathname === "/mock-authorize") {
      mockAuthorize(req, res);
      return;
    }
    if (url.pathname === "/mock-cancel") {
      mockCancel(req, res);
      return;
    }
    if (url.pathname === "/callback") {
      await realCallback(req, res);
      return;
    }
    if (url.pathname === "/verify") {
      verify(req, res);
      return;
    }
    if (url.pathname === "/profile") {
      await profile(req, res);
      return;
    }
    res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Not Found");
  } catch (error) {
    console.error("[dev-steam-relay] request failed:", error);
    json(res, { success: false, error: "internal error" }, 500);
  }
});

server.listen(PORT, HOST, () => {
  console.log(
    `[dev-steam-relay] listening on http://${HOST}:${PORT} (mode: ${MODE})${STEAM_API_KEY ? "" : " — STEAM_API_KEY 未配置，资料使用默认昵称"}`,
  );
});

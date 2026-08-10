#!/usr/bin/env node
/**
 * LumiForum — 开发环境配置自动选择器（仅本地开发使用）
 *
 * 行为：
 *   - 自动加载 .env.development（开发环境配置）；若不存在则回退 .env。
 *   - 不覆盖已存在的环境变量 —— 因此 CI / 生产部署注入的环境永远优先，
 *     本脚本在生产构建（pnpm build*）中不会被调用，也不影响正式环境。
 *
 * 用法（package.json dev 脚本）：
 *   node scripts/dev-env.mjs <command> [args...]
 */
import { existsSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const candidates = [".env.development", ".env"];
const chosen = candidates.find((file) => existsSync(path.join(repoRoot, file)));

if (!chosen) {
  console.error(
    "[dev-env] 未找到开发环境配置：缺少 .env.development（或 .env）。\n" +
      "[dev-env] 请复制仓库中的 .env.development（如无则参考 .env.example）后重试。",
  );
  process.exit(1);
}

// 解析 dotenv 格式（支持 export 前缀与引号），不处理变量插值。
for (const rawLine of readFileSync(path.join(repoRoot, chosen), "utf8").split(/\r?\n/)) {
  const line = rawLine.trim();
  if (!line || line.startsWith("#")) continue;
  const match = line.match(/^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/);
  if (!match) continue;
  const [, key, rawValue] = match;
  let value = rawValue.trim();
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    value = value.slice(1, -1);
  }
  if (!(key in process.env)) {
    process.env[key] = value;
  }
}

console.log(`[dev-env] 已加载开发环境配置: ${chosen}`);
const result = spawnSync(process.argv[2], process.argv.slice(3), {
  stdio: "inherit",
  env: process.env,
});
process.exit(result.status ?? 1);

/**
 * CNGOKZ Steam Auth Relay — Cloudflare Worker
 * ============================================
 * 论坛服务器无法直连 Steam 网络，所有 Steam 交互经本 Worker 中继。
 *
 * 部署：
 *   1) 在 Cloudflare 创建 KV namespace（如 STEAM_TOKENS），并在 Worker 设置
 *      中绑定变量名 STEAM_TOKENS（用于一次性 token 存储，TTL 5 分钟）。
 *   2) 可选变量：
 *        - STEAM_API_KEY            Steam Web API Key（用于同步昵称/头像；
 *                                    不配置时仅返回 steamid）
 *        - ALLOWED_CALLBACK_HOSTS   允许回跳的论坛回调域名白名单，逗号分隔，
 *                                    默认 "chatapi.cngokz.com"
 *
 * 登录流程（防伪造）：
 *   Steam → Worker /callback（验证 OpenID）→ 生成一次性 token（KV 单次消费）
 *   → 302 回论坛 /auth/steam/callback?token=xxx&state=yyy
 *   → 论坛后端 GET Worker /verify?token=xxx 换取 SteamID（token 用后即焚），
 *     无法通过伪造 ?steamid=xxx 绕过登录。
 */
export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    const AUTH_DOMAIN = "https://cngokz-steam-auth.iquankz.cn";
    const DEFAULT_CALLBACK = "https://chatapi.cngokz.com/auth/steam/callback";
    const TOKEN_TTL_SECONDS = 300;

    const json = (body, status = 200) =>
      new Response(JSON.stringify(body), {
        status,
        headers: { "Content-Type": "application/json" },
      });

    /*
     * 健康检查
     */
    if (url.pathname === "/") {
      return json({ service: "CNGOKZ Steam Auth", status: "running" });
    }

    /*
     * 开始 Steam 登录：
     *   /login?mode=login|bind&state=<论坛 state>&return_to=<论坛回调地址>
     */
    if (url.pathname === "/login") {
      const state = url.searchParams.get("state") || "";
      const returnTo = resolveCallback(url.searchParams.get("return_to"), env);

      // Steam 会原样回跳 return_to（含查询参数），state/return_to 随 OpenID
      // 往返后由 /callback 读取。
      const workerCallback = new URL(`${AUTH_DOMAIN}/callback`);
      if (state) workerCallback.searchParams.set("state", state);
      workerCallback.searchParams.set("return_to", returnTo);

      const steam = new URL("https://steamcommunity.com/openid/login");
      const params = {
        "openid.ns": "http://specs.openid.net/auth/2.0",
        "openid.mode": "checkid_setup",
        "openid.return_to": workerCallback.toString(),
        "openid.realm": AUTH_DOMAIN,
        "openid.identity": "http://specs.openid.net/auth/2.0/identifier_select",
        "openid.claimed_id": "http://specs.openid.net/auth/2.0/identifier_select",
      };
      for (const [key, value] of Object.entries(params)) {
        steam.searchParams.set(key, value);
      }
      return Response.redirect(steam.toString(), 302);
    }

    /*
     * Steam 回调：验证 OpenID 断言 → 生成一次性 token → 跳回论坛
     */
    if (url.pathname === "/callback") {
      const params = new URLSearchParams(url.search);
      const returnTo = params.get("return_to") || DEFAULT_CALLBACK;
      const state = params.get("state") || "";

      // 用户取消授权
      if (params.get("openid.mode") === "cancel") {
        return redirectWith(returnTo, { error: "steam_access_denied", state });
      }

      // 向 Steam 验证断言
      params.set("openid.mode", "check_authentication");
      const verify = await fetch("https://steamcommunity.com/openid/login", {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: params.toString(),
      });
      const result = await verify.text();
      if (!result.includes("is_valid:true")) {
        return redirectWith(returnTo, { error: "steam_auth_failed", state });
      }

      const claimed = params.get("openid.claimed_id") || "";
      const steamid = claimed.split("/").pop() || "";
      if (!/^\d{17}$/.test(steamid)) {
        return redirectWith(returnTo, { error: "steam_auth_failed", state });
      }

      // 生成一次性 token（KV 存储、TTL 5 分钟，/verify 取用后即焚）
      const token = crypto.randomUUID().replaceAll("-", "");
      const record = { steamid };
      if (env.STEAM_API_KEY) {
        const profile = await fetchSteamProfile(env.STEAM_API_KEY, steamid);
        if (profile) Object.assign(record, profile);
      }
      await env.STEAM_TOKENS.put(token, JSON.stringify(record), {
        expirationTtl: TOKEN_TTL_SECONDS,
      });

      return redirectWith(returnTo, { token, state });
    }

    /*
     * 一次性 token 换取 SteamID（论坛后端调用，单次有效）
     *   GET /verify?token=xxx
     */
    if (url.pathname === "/verify") {
      const token = url.searchParams.get("token") || "";
      if (!token) return json({ success: false, error: "missing token" }, 400);
      const record = await env.STEAM_TOKENS.get(token);
      if (!record) return json({ success: false, error: "invalid or expired token" }, 401);
      await env.STEAM_TOKENS.delete(token); // 单次消费，防重放
      return json({ success: true, ...JSON.parse(record) });
    }

    /*
     * 按 SteamID 同步资料（登录后刷新昵称/头像）
     *   GET /profile?steamid=xxx
     */
    if (url.pathname === "/profile") {
      const steamid = url.searchParams.get("steamid") || "";
      if (!/^\d{17}$/.test(steamid)) {
        return json({ success: false, error: "invalid steamid" }, 400);
      }
      if (!env.STEAM_API_KEY) {
        return json({ success: false, error: "STEAM_API_KEY not configured" }, 501);
      }
      const profile = await fetchSteamProfile(env.STEAM_API_KEY, steamid);
      if (!profile) {
        return json({ success: false, error: "steam profile not found" }, 404);
      }
      return json({ success: true, steamid, ...profile });
    }

    return new Response("Not Found", { status: 404 });
  },
};

/** 校验 return_to 是否在回调域名白名单内，防止被用作开放重定向。 */
function resolveCallback(returnTo, env) {
  const allowed = (env.ALLOWED_CALLBACK_HOSTS || "chatapi.cngokz.com")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  try {
    const parsed = new URL(returnTo);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") {
      if (allowed.includes(parsed.host)) return returnTo;
    }
  } catch {
    // fall through to default
  }
  return "https://chatapi.cngokz.com/auth/steam/callback";
}

/** 302 跳转到论坛回调地址并附带查询参数。 */
function redirectWith(base, query) {
  const target = new URL(base);
  for (const [key, value] of Object.entries(query)) {
    if (value) target.searchParams.set(key, value);
  }
  return Response.redirect(target.toString(), 302);
}

/** 通过 Steam Web API 获取玩家资料（需要 STEAM_API_KEY）。 */
async function fetchSteamProfile(apiKey, steamid) {
  try {
    const response = await fetch(
      `https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key=${encodeURIComponent(apiKey)}&steamids=${steamid}`,
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
  } catch {
    return null;
  }
}

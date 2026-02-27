use crate::constants::KV_BINDING;
use crate::models::{ApiResponse, RateLimitState, SessionData};
use crate::response::json_with_status;
use crate::utils::extract_ip;
use worker::js_sys::Date;
use worker::{Env, HttpResponse, Request};

pub async fn require_auth(
    req: &Request,
    env: &Env,
) -> std::result::Result<SessionData, HttpResponse> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok()
        .flatten()
        .unwrap_or_default();

    if !auth_header.starts_with("Bearer ") {
        return Err(
            json_with_status(
                401,
                &ApiResponse {
                    success: false,
                    message: "未登录或 token 缺失".to_string(),
                },
                "*",
            )
            .unwrap_or_else(|_| {
                worker::HttpResponse::try_from(worker::Response::error("Unauthorized", 401).unwrap())
                    .unwrap()
            }),
        );
    }

    let token = auth_header.trim_start_matches("Bearer ").trim();
    if token.is_empty() {
        return Err(
            json_with_status(
                401,
                &ApiResponse {
                    success: false,
                    message: "token 无效".to_string(),
                },
                "*",
            )
            .unwrap_or_else(|_| {
                worker::HttpResponse::try_from(worker::Response::error("Unauthorized", 401).unwrap())
                    .unwrap()
            }),
        );
    }

    let kv = match env.kv(KV_BINDING) {
        Ok(v) => v,
        Err(_) => {
            return Err(
                json_with_status(
                    500,
                    &ApiResponse {
                        success: false,
                        message: "未配置 KV_LIMITER 绑定".to_string(),
                    },
                    "*",
                )
                .unwrap_or_else(|_| {
                    worker::HttpResponse::try_from(worker::Response::error("Server Error", 500).unwrap())
                        .unwrap()
                }),
            )
        }
    };

    let session = kv
        .get(&format!("session:{}", token))
        .json::<SessionData>()
        .await
        .ok()
        .flatten();

    let Some(session) = session else {
        return Err(
            json_with_status(
                401,
                &ApiResponse {
                    success: false,
                    message: "登录已过期，请重新登录".to_string(),
                },
                "*",
            )
            .unwrap_or_else(|_| {
                worker::HttpResponse::try_from(worker::Response::error("Unauthorized", 401).unwrap())
                    .unwrap()
            }),
        );
    };

    Ok(session)
}

pub async fn apply_rate_limit(
    req: &Request,
    env: &Env,
    prefix: &str,
) -> std::result::Result<(), String> {
    let max_per_window: u32 = env
        .var("RATE_LIMIT_MAX")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(5);
    let window_secs: i64 = env
        .var("RATE_LIMIT_WINDOW_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(60);

    let ip = extract_ip(req);

    if let Ok(rate_limiter) = env.rate_limiter("REG_RATE_LIMITER") {
        let outcome = rate_limiter
            .limit(format!("{}:{}", prefix, ip))
            .await
            .map_err(|_| "RateLimiter 调用失败".to_string())?;
        if !outcome.success {
            return Err("请求过于频繁，请稍后再试".to_string());
        }
    }

    let key = format!("{}:{}", prefix, ip);
    let now_sec = (Date::now() / 1000.0).floor() as i64;
    let kv = env
        .kv(KV_BINDING)
        .map_err(|_| "未配置 KV_LIMITER 绑定".to_string())?;

    let current = kv
        .get(&key)
        .json::<RateLimitState>()
        .await
        .map_err(|e| format!("KV 读取失败: {e}"))?;

    let mut state = current.unwrap_or(RateLimitState {
        count: 0,
        window_start_sec: now_sec,
    });

    if now_sec - state.window_start_sec >= window_secs {
        state.window_start_sec = now_sec;
        state.count = 0;
    }

    if state.count >= max_per_window {
        return Err("请求过于频繁，请稍后再试".to_string());
    }

    state.count += 1;
    kv.put(&key, &state)
        .map_err(|e| format!("KV 写入失败: {e}"))?
        .expiration_ttl((window_secs * 2) as u64)
        .execute()
        .await
        .map_err(|e| format!("KV 写入失败: {e}"))?;

    Ok(())
}

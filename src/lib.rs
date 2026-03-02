mod auth;
mod constants;
mod handlers;
mod models;
mod response;
mod utils;

use std::convert::TryFrom;
use worker::*;

use models::ApiResponse;
use response::{empty_with_status, html_with_status, json_with_status};

fn infer_request_origin(req: &Request) -> Option<String> {
    let host = req
        .headers()
        .get("Host")
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())?;

    let proto = req
        .headers()
        .get("X-Forwarded-Proto")
        .ok()
        .flatten()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| v == "http" || v == "https")
        .unwrap_or_else(|| "https".to_string());

    Some(format!("{}://{}", proto, host))
}

fn normalize_origin_like(value: &str) -> String {
    value.trim().trim_end_matches('/').to_lowercase()
}

fn is_local_dev_origin(origin: &str) -> bool {
    let origin = normalize_origin_like(origin);
    for host in ["localhost", "127.0.0.1"] {
        for scheme in ["http", "https"] {
            let base = format!("{}://{}", scheme, host);
            if origin == base || origin.starts_with(&format!("{}:", base)) {
                return true;
            }
        }
    }
    origin == "null"
}

fn cors_origin_matches(allowed: &str, origin: &str) -> bool {
    let allowed = normalize_origin_like(allowed);
    let origin = normalize_origin_like(origin);

    if allowed == origin {
        return true;
    }

    if let Some(prefix) = allowed.strip_suffix(":*") {
        return origin == prefix || origin.starts_with(&format!("{}:", prefix));
    }

    if allowed == "localhost" || allowed == "127.0.0.1" {
        for scheme in ["http", "https"] {
            let base = format!("{}://{}", scheme, allowed);
            if origin == base || origin.starts_with(&format!("{}:", base)) {
                return true;
            }
        }
    }

    for host in ["localhost", "127.0.0.1"] {
        for scheme in ["http", "https"] {
            let base = format!("{}://{}", scheme, host);
            if allowed == base {
                return origin == base || origin.starts_with(&format!("{}:", base));
            }
        }
    }

    false
}

fn resolve_cors_origin(req: &Request, env: &Env) -> (String, bool) {
    let request_origin = req
        .headers()
        .get("Origin")
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let configured = env
        .var("CORS_ORIGIN")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_default();

    let allowlist: Vec<String> = configured
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty() && *v != "*")
        .map(|v| v.to_string())
        .collect();

    if !allowlist.is_empty() {
        if let Some(origin) = request_origin {
            if is_local_dev_origin(&origin) {
                return (origin, true);
            }
            if allowlist.iter().any(|v| cors_origin_matches(v, &origin)) {
                return (origin, true);
            }
            return (allowlist[0].clone(), false);
        }
        return (allowlist[0].clone(), true);
    }

    let fallback_origin =
        infer_request_origin(req).unwrap_or_else(|| "https://localhost".to_string());
    if let Some(origin) = request_origin {
        let allowed = origin == fallback_origin;
        return (fallback_origin, allowed);
    }

    (fallback_origin, true)
}

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<HttpResponse> {
    let mut req = Request::try_from(req)?;
    let path = req.path();
    let method = req.method();
    let (mut cors_origin, mut origin_allowed) = resolve_cors_origin(&req, &env);

    let host_header = req
        .headers()
        .get("Host")
        .ok()
        .flatten()
        .unwrap_or_default()
        .to_lowercase();
    let req_origin = req
        .headers()
        .get("Origin")
        .ok()
        .flatten()
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    let url_host = req
        .url()
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let host_is_local = host_header.starts_with("localhost")
        || host_header.starts_with("127.0.0.1")
        || url_host.starts_with("localhost")
        || url_host.starts_with("127.0.0.1");

    worker::console_log!(
        "[CORS] path={} origin={} host={} url_host={} host_is_local={} origin_allowed={}",
        path,
        req_origin,
        host_header,
        url_host,
        host_is_local,
        origin_allowed
    );

    if host_is_local && path.starts_with("/api/") {
        origin_allowed = true;
        if !req_origin.is_empty() {
            cors_origin = req_origin.clone();
        } else {
            cors_origin = "http://localhost".to_string();
        }
    }

    if path.starts_with("/api/") && !origin_allowed {
        let _ = req.text().await;
        worker::console_log!("[CORS] REJECTED origin={} host={}", req_origin, host_header);
        return json_with_status(
            403,
            &ApiResponse {
                success: false,
                message: format!(
                    "Origin not allowed (origin={}, host={})",
                    req_origin, host_header
                ),
            },
            &cors_origin,
        );
    }

    if method == Method::Options && path.starts_with("/api/") {
        return empty_with_status(204, &cors_origin);
    }

    if method == Method::Get && path == "/" {
        let html = include_str!("../public/app.html");
        return html_with_status(200, html, &cors_origin);
    }

    if method == Method::Get && path == "/presentation" {
        let html = include_str!("../public/presentation.html");
        return html_with_status(200, html, &cors_origin);
    }

    match (method, path.as_str()) {
        (Method::Post, "/api/register") => handlers::register(req, &env, &cors_origin).await,
        (Method::Post, "/api/login") => handlers::login(req, &env, &cors_origin).await,
        (Method::Get, "/api/me") => handlers::me(req, &env, &cors_origin).await,
        (Method::Post, "/api/workspace/save") => {
            handlers::save_workspace(req, &env, &cors_origin).await
        }
        (Method::Get, "/api/workspace/latest") => {
            handlers::latest_workspace(req, &env, &cors_origin).await
        }
        (Method::Post, "/api/png/generate") => {
            handlers::generate_png(req, &env, &cors_origin).await
        }
        (Method::Post, "/api/prescription/generate") => {
            handlers::generate_prescription(req, &env, &cors_origin).await
        }
        (Method::Get, "/api/prescription/random") => handlers::prescription_random(&cors_origin),
        (Method::Get, "/api/health") => json_with_status(
            200,
            &ApiResponse {
                success: true,
                message: "ok".to_string(),
            },
            &cors_origin,
        ),
        _ => json_with_status(
            404,
            &ApiResponse {
                success: false,
                message: "Not Found".to_string(),
            },
            &cors_origin,
        ),
    }
}

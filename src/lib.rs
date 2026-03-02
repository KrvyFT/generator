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
            if allowlist.iter().any(|v| v == &origin) {
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
    let req = Request::try_from(req)?;
    let path = req.path();
    let method = req.method();
    let (cors_origin, origin_allowed) = resolve_cors_origin(&req, &env);

    if path.starts_with("/api/") && !origin_allowed {
        return json_with_status(
            403,
            &ApiResponse {
                success: false,
                message: "Origin not allowed".to_string(),
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

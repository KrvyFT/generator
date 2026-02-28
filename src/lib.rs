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

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<HttpResponse> {
    let req = Request::try_from(req)?;
    let path = req.path();
    let method = req.method();
    let cors_origin = env
        .var("CORS_ORIGIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "*".to_string());

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
        (Method::Post, "/api/pdf/generate") => {
            handlers::generate_pdf(req, &env, &cors_origin).await
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

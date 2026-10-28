use actix_multipart::form::MultipartFormConfig;
use actix_web::{get, http::StatusCode, web, HttpResponse};
use serde_json::json;

mod auth;
mod hackatime;
mod submit;

pub const AUTH_CALLBACK: &str = "/api/callback";
pub const HACKATIME_CALLBACK: &str = "/api/hackatime/callback";

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.app_data(
        MultipartFormConfig::default()
            .total_limit(submit::MAX_SCREENSHOT_BYTES + 500_000)
            .memory_limit(500_000),
    )
    .service(health)
    .service(auth::hca_login)
    .service(auth::hca_callback)
    .service(auth::hackatime_login)
    .service(auth::hackatime_callback)
    .service(auth::me)
    .service(hackatime::projects)
    .service(submit::submit);
}

pub fn fail(status: StatusCode, message: &str) -> HttpResponse {
    HttpResponse::build(status).json(json!({ "error": message }))
}

pub async fn not_found() -> HttpResponse {
    fail(StatusCode::NOT_FOUND, "Not found")
}

pub fn site_url(path: &str) -> String {
    let base = hc_helper::keys::base_url();
    let host = base
        .rsplit_once(':')
        .map_or(base.as_str(), |(host, _)| host);
    let scheme = match host {
        "localhost" | "127.0.0.1" | "[::1]" => "http",
        _ => "https",
    };
    format!("{scheme}://{base}{path}")
}

fn is_unauthorized(err: &anyhow::Error) -> bool {
    err.downcast_ref::<reqwest::Error>()
        .and_then(reqwest::Error::status)
        .is_some_and(|status| matches!(status.as_u16(), 401 | 403))
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}

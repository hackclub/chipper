use std::time::Instant;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::{header, Method, StatusCode};
use actix_web::middleware::{from_fn, DefaultHeaders, Next};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer};
use rust_embed::RustEmbed;
use tracing::{error, event, info, warn, Level};
use tracing_subscriber::EnvFilter;

mod airtable;
mod api;

const REQUIRED_ENV: [&str; 8] = [
    "BASE_URL",
    "HCA_CLIENT_ID",
    "HCA_CLIENT_SECRET",
    "HACKATIME_CLIENT_ID",
    "HACKATIME_CLIENT_SECRET",
    "AIRTABLE_TOKEN",
    "AIRTABLE_BASE_ID",
    "AIRTABLE_TABLE",
];

const CSP: &str = "default-src 'self'; \
    script-src 'self' 'unsafe-inline' https://plausible.io; \
    style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
    font-src 'self' https://fonts.gstatic.com; \
    img-src 'self' data: blob: https:; \
    connect-src 'self' https://plausible.io; \
    frame-ancestors 'none'; \
    base-uri 'self'; \
    object-src 'none'";

#[derive(RustEmbed)]
#[folder = "frontend/build/"]
struct Frontend;

async fn frontend(req: HttpRequest) -> HttpResponse {
    let path = req.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let (status, file) =
        match Frontend::get(path).or_else(|| Frontend::get(&format!("{path}.html"))) {
            Some(file) => (StatusCode::OK, file),
            None => (
                StatusCode::NOT_FOUND,
                Frontend::get("200.html").expect("200.html is embedded"),
            ),
        };
    HttpResponseBuilder::new(status)
        .content_type(file.metadata.mimetype().to_owned())
        .body(file.data.into_owned())
}

async fn reject_cross_site(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<actix_web::body::BoxBody>, Error> {
    let header = |name| req.headers().get(name).and_then(|v| v.to_str().ok());
    let cross_site = match (header("sec-fetch-site"), header("origin")) {
        (Some(site), _) => site != "same-origin",
        (None, Some(origin)) => origin.split_once("://").map(|(_, host)| host) != header("host"),
        (None, None) => false,
    };
    if req.method() != Method::GET && req.method() != Method::HEAD && cross_site {
        warn!(path = req.path(), "rejected cross-site request");
        let res = api::fail(StatusCode::FORBIDDEN, "Cross-site request");
        return Ok(req.into_response(res));
    }
    Ok(next.call(req).await?.map_into_boxed_body())
}

async fn log_requests(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let method = req.method().to_string();
    let path = req.path().to_string();
    let peer = hc_helper::client_ip(req.request());
    let start = Instant::now();

    let res = next.call(req).await?;

    let status = res.status().as_u16();
    let ms = format!("{:.2}", start.elapsed().as_secs_f64() * 1000.0);
    match status {
        500.. => event!(Level::ERROR, %method, %path, status, %ms, %peer, "request"),
        400..=499 => event!(Level::WARN, %method, %path, status, %ms, %peer, "request"),
        _ => event!(Level::INFO, %method, %path, status, %ms, %peer, "request"),
    }
    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let dotenv = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(true)
        .with_target(true)
        .init();

    match dotenv {
        Ok(path) => info!(path = %path.display(), "loaded env file"),
        Err(e) if e.not_found() => info!("no .env file, using process environment"),
        Err(e) => error!(error = %e, "couldn't load .env"),
    }
    let missing: Vec<_> = REQUIRED_ENV
        .into_iter()
        .filter(|key| std::env::var(key).map_or(true, |v| v.trim().is_empty()))
        .collect();
    if missing.is_empty() {
        info!(
            auth = api::site_url(api::AUTH_CALLBACK),
            hackatime = api::site_url(api::HACKATIME_CALLBACK),
            "OAuth redirect URIs"
        );
    } else {
        error!(
            ?missing,
            "MISSING ENV VARS, SIGN IN AND SUBMISSIONS WILL FAIL"
        );
    }
    if let Err(e) = hc_helper::airtable::spawn_airtable_queue_handler() {
        error!(error = %format!("{e:#}"), "couldn't start Airtable queue, SUBMISSIONS WILL FAIL");
    }
    actix_web::rt::spawn(airtable::ensure_schema());

    let addr = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());

    info!(version = env!("CARGO_PKG_VERSION"), "starting chipper");
    let mut files = 0usize;
    let mut bytes = 0usize;
    for name in Frontend::iter() {
        if let Some(file) = Frontend::get(&name) {
            files += 1;
            bytes += file.data.len();
        }
    }
    if files == 0 {
        warn!("no frontend files embedded");
    }
    info!(files, bytes, "frontend embedded");
    info!(%addr, "binding");

    let server = HttpServer::new(|| {
        App::new()
            .wrap(
                DefaultHeaders::new()
                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                    .add((header::X_FRAME_OPTIONS, "DENY"))
                    .add((header::REFERRER_POLICY, "strict-origin-when-cross-origin"))
                    .add((header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"))
                    .add((header::CONTENT_SECURITY_POLICY, CSP)),
            )
            .wrap(from_fn(log_requests))
            .service(
                web::scope("/api")
                    .wrap(DefaultHeaders::new().add((header::CACHE_CONTROL, "no-store")))
                    .wrap(from_fn(reject_cross_site))
                    .configure(api::configure)
                    .default_service(web::to(api::not_found)),
            )
            .default_service(web::to(frontend))
    })
    .bind(&addr)
    .map_err(|e| {
        error!(%addr, error = %e, "failed to bind");
        e
    })?;

    info!(%addr, url = api::site_url("/"), "listening");
    server.run().await?;
    info!("server stopped");
    Ok(())
}

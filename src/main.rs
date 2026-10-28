use std::time::Instant;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::{from_fn, Next};
use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use rust_embed::RustEmbed;
use serde::Serialize;
use tracing::{error, event, info, warn, Level};
use tracing_subscriber::EnvFilter;

#[derive(RustEmbed)]
#[folder = "frontend/build/"]
struct Frontend;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[get("/health")]
async fn health() -> impl Responder {
    web::Json(Health { status: "ok" })
}

async fn frontend(req: HttpRequest) -> HttpResponse {
    let path = req.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let file = Frontend::get(path)
        .or_else(|| Frontend::get("200.html"))
        .expect("200.html is embedded");
    HttpResponse::Ok()
        .content_type(file.metadata.mimetype().to_owned())
        .body(file.data.into_owned())
}

async fn log_requests(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().to_string();
    let peer = req.peer_addr().map_or_else(|| "-".to_string(), |a| a.to_string());
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(true)
        .with_target(true)
        .init();

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
            .wrap(from_fn(log_requests))
            .service(web::scope("/api").service(health))
            .default_service(web::to(frontend))
    })
    .bind(&addr)
    .map_err(|e| {
        error!(%addr, error = %e, "failed to bind");
        e
    })?;

    info!(%addr, "listening, open http://{addr}");
    server.run().await?;
    info!("server stopped");
    Ok(())
}

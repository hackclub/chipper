use std::{
    net::{IpAddr, Ipv6Addr},
    time::Duration,
};

use actix_governor::{
    governor::{
        clock::{Clock, DefaultClock, QuantaInstant},
        middleware::NoOpMiddleware,
        NotUntil,
    },
    GovernorConfig, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError,
};
use actix_web::{
    dev::ServiceRequest,
    http::{header, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use serde_json::json;

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct ClientIp;

impl KeyExtractor for ClientIp {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<IpAddr, Self::KeyExtractionError> {
        let ip = req
            .headers()
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok())
            .or_else(|| req.peer_addr().map(|addr| addr.ip()))
            .ok_or_else(|| SimpleKeyExtractionError::new("Couldn't identify client"))?;
        Ok(match ip {
            IpAddr::V6(v6) => IpAddr::V6(Ipv6Addr::from(u128::from(v6) & (!0u128 << 64))),
            v4 => v4,
        })
    }

    fn exceed_rate_limit_response(
        &self,
        negative: &NotUntil<QuantaInstant>,
        _response: HttpResponseBuilder,
    ) -> HttpResponse {
        let wait = negative.wait_time_from(DefaultClock::default().now());
        HttpResponse::build(StatusCode::TOO_MANY_REQUESTS)
            .insert_header((header::RETRY_AFTER, wait.as_secs().max(1)))
            .json(json!({ "error": "Too many requests, please wait a moment and try again" }))
    }
}

pub fn config() -> GovernorConfig<ClientIp, NoOpMiddleware<QuantaInstant>> {
    let config = GovernorConfigBuilder::default()
        .key_extractor(ClientIp)
        .milliseconds_per_request(500)
        .burst_size(60)
        .finish()
        .expect("rate limit quota is valid");

    let limiter = config.limiter();
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(CLEANUP_INTERVAL).await;
            limiter.retain_recent();
            limiter.shrink_to_fit();
        }
    });

    config
}

use actix_web::{get, web::Query, HttpRequest, HttpResponse};
use hc_helper::{
    auth::{
        data::get_auth_data_with_token,
        login::{self as hca_oauth, get_auth_token_with_handling},
    },
    hackatime::{
        data::get_hackatime_user_with_token,
        login::{self as hackatime_oauth, get_hackatime_token_with_handling},
    },
};
use serde::Serialize;
use tracing::warn;

use super::{site_url, AUTH_CALLBACK, HACKATIME_CALLBACK};

type NoTokenHook = fn(String) -> std::future::Ready<()>;

#[get("/login")]
pub async fn hca_login(req: HttpRequest) -> HttpResponse {
    let scopes = hca_oauth::Scopes {
        openid: false,
        profile: false,
        email: false,
        name: true,
        slack_id: true,
        phone_number: true,
        verification_status: true,
        basic_info: true,
        addresses: true,
    };
    hca_oauth::handle_login(&req, scopes, site_url(AUTH_CALLBACK)).await
}

#[get("/callback")]
pub async fn hca_callback(req: HttpRequest, query: Query<hca_oauth::CallbackArgs>) -> HttpResponse {
    hca_oauth::handle_callback::<NoTokenHook>(
        &req,
        query,
        site_url(AUTH_CALLBACK),
        "/submit".into(),
        None,
        true,
    )
    .await
}

#[get("/hackatime/login")]
pub async fn hackatime_login(req: HttpRequest) -> HttpResponse {
    let scopes = hackatime_oauth::Scopes {
        profile: true,
        read: true,
        admin: false,
    };
    hackatime_oauth::handle_login(&req, scopes, site_url(HACKATIME_CALLBACK), true).await
}

#[get("/hackatime/callback")]
pub async fn hackatime_callback(
    req: HttpRequest,
    query: Query<hackatime_oauth::CallbackArgs>,
) -> HttpResponse {
    hackatime_oauth::handle_callback::<NoTokenHook>(
        &req,
        query,
        site_url(HACKATIME_CALLBACK),
        "/submit".into(),
        None,
        true,
    )
    .await
}

#[derive(Serialize)]
struct Me {
    signed_in: bool,
    first_name: String,
    hackatime_linked: bool,
}

#[get("/me")]
pub async fn me(req: HttpRequest) -> HttpResponse {
    let auth = match get_auth_token_with_handling(&req) {
        Some(token) => get_auth_data_with_token(&token)
            .await
            .inspect_err(
                |e| warn!(error = %format!("{e:#}"), "couldn't fetch Hack Club Auth profile"),
            )
            .ok(),
        None => None,
    };
    let hackatime_linked = match get_hackatime_token_with_handling(&req) {
        Some(token) => get_hackatime_user_with_token(&token)
            .await
            .inspect_err(|e| warn!(error = %format!("{e:#}"), "couldn't fetch Hackatime user"))
            .is_ok(),
        None => false,
    };

    HttpResponse::Ok().json(Me {
        signed_in: auth.is_some(),
        first_name: auth.map(|a| a.first_name).unwrap_or_default(),
        hackatime_linked,
    })
}

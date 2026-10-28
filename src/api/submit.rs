use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex, PoisonError},
};

use actix_multipart::{
    form::{tempfile::TempFile, text::Text, MultipartForm},
    MultipartError,
};
use actix_web::{error::PayloadError, http::StatusCode, post, web, HttpRequest, HttpResponse};
use hc_helper::{
    airtable::find_records,
    auth::{data::get_auth_data_with_token, login::get_auth_token_with_handling},
    hackatime::{
        data::{get_hackatime_project_summaries_with_token, get_hackatime_user_with_token},
        login::get_hackatime_token_with_handling,
    },
    submission::create_unified_with_token,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

use super::{fail, is_unauthorized};
use crate::airtable;

const SECONDS_PER_HOUR: f64 = 3600.0;
pub const MAX_SCREENSHOT_BYTES: usize = 5_000_000;
const MAX_DESCRIPTION_CHARS: usize = 10_000;

static IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

struct InFlight(String);

impl InFlight {
    fn claim(id: &str) -> Option<Self> {
        let mut ids = IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner);
        ids.insert(id.to_string()).then(|| Self(id.to_string()))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.0);
    }
}

fn is_overflow(err: &MultipartError) -> bool {
    match err {
        MultipartError::Payload(PayloadError::Overflow) => true,
        MultipartError::Field { source, .. } => source.as_error().is_some_and(is_overflow),
        _ => false,
    }
}

fn prize_tier(hours: f64) -> Option<&'static str> {
    match hours {
        h if h >= 30.0 => Some(airtable::PRIZE_TIERS[2]),
        h if h >= 25.0 => Some(airtable::PRIZE_TIERS[1]),
        h if h >= 10.0 => Some(airtable::PRIZE_TIERS[0]),
        _ => None,
    }
}

#[derive(MultipartForm)]
pub struct SubmitForm {
    repository_url: Text<String>,
    code_url: Text<String>,
    description: Text<String>,
    projects: Vec<Text<String>>,
    #[multipart(limit = "5MB")]
    screenshot: TempFile,
}

#[derive(Serialize)]
struct ExtraFields {
    #[serde(rename = "Prize Tier")]
    prize_tier: &'static str,
    #[serde(rename = "HCA ID")]
    hca_id: String,
    #[serde(rename = "Hackatime Projects")]
    hackatime_projects: String,
    #[serde(rename = "Slack ID")]
    slack_id: String,
    #[serde(rename = "Phone Number")]
    phone_number: String,
    #[serde(rename = "Justification - Submitter Hackatime ID")]
    hackatime_id: String,
}

#[derive(Deserialize, Default)]
struct EarlierSubmission {
    #[serde(rename = "Hackatime Projects", default)]
    hackatime_projects: String,
    #[serde(rename = "Screenshot", default)]
    screenshot: Vec<serde_json::Value>,
}

fn sniff_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    match bytes {
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some(("image/png", "png")),
        [0xFF, 0xD8, 0xFF, ..] => Some(("image/jpeg", "jpg")),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => Some(("image/gif", "gif")),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => {
            Some(("image/webp", "webp"))
        }
        _ => None,
    }
}

fn is_http_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

#[post("/submit")]
pub async fn submit(
    req: HttpRequest,
    form: Result<MultipartForm<SubmitForm>, actix_web::Error>,
) -> HttpResponse {
    let Some(auth_token) = get_auth_token_with_handling(&req) else {
        return fail(
            StatusCode::UNAUTHORIZED,
            "Sign in with Hack Club Auth first",
        );
    };
    let Some(hackatime_token) = get_hackatime_token_with_handling(&req) else {
        return fail(
            StatusCode::UNAUTHORIZED,
            "Link your Hackatime account first",
        );
    };
    let form = match form {
        Ok(MultipartForm(form)) => form,
        Err(e) => {
            let err = e.as_error::<MultipartError>();
            return if err.is_some_and(is_overflow) {
                fail(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Screenshot must be 5 MB or smaller",
                )
            } else if let Some(MultipartError::MissingField(_)) = err {
                fail(StatusCode::BAD_REQUEST, "Fill in every field")
            } else {
                fail(
                    StatusCode::BAD_REQUEST,
                    "Couldn't read your submission, please try again",
                )
            };
        }
    };

    let repository_url = form.repository_url.trim().to_string();
    let code_url = form.code_url.trim().to_string();
    let description = form.description.replace("\r\n", "\n").trim().to_string();
    if !is_http_url(&repository_url) {
        return fail(
            StatusCode::BAD_REQUEST,
            "Repository URL must be an http(s) link",
        );
    }
    if !is_http_url(&code_url) {
        return fail(StatusCode::BAD_REQUEST, "Code URL must be an http(s) link");
    }
    if description.is_empty() {
        return fail(StatusCode::BAD_REQUEST, "Add a description");
    }
    if description.chars().count() > MAX_DESCRIPTION_CHARS {
        return fail(
            StatusCode::BAD_REQUEST,
            "Description must be 10,000 characters or fewer",
        );
    }

    let path = form.screenshot.file.path().to_owned();
    let bytes = match web::block(move || std::fs::read(path)).await {
        Ok(Ok(bytes)) if !bytes.is_empty() => bytes,
        Ok(Ok(_)) => return fail(StatusCode::BAD_REQUEST, "Add a screenshot"),
        Ok(Err(e)) => {
            error!(error = %e, "couldn't read uploaded screenshot");
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't read your screenshot",
            );
        }
        Err(e) => {
            error!(error = %e, "couldn't read uploaded screenshot");
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't read your screenshot",
            );
        }
    };

    let Some((content_type, extension)) = sniff_image(&bytes) else {
        return fail(
            StatusCode::BAD_REQUEST,
            "Screenshot must be a PNG, JPEG, GIF or WebP image",
        );
    };
    let stem: String = form
        .screenshot
        .file_name
        .as_deref()
        .and_then(|name| name.rsplit_once('.').map(|(stem, _)| stem))
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .take(100)
        .collect();
    let stem = match stem.trim() {
        "" => "screenshot",
        stem => stem,
    };
    let filename = format!("{stem}.{extension}");

    let mut names: Vec<String> = Vec::new();
    for name in form.projects {
        let name = name.trim().to_string();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        return fail(
            StatusCode::BAD_REQUEST,
            "Tick at least one Hackatime project",
        );
    }

    let summaries = match get_hackatime_project_summaries_with_token(&hackatime_token).await {
        Ok(x) => x,
        Err(e) if is_unauthorized(&e) => {
            return fail(
                StatusCode::UNAUTHORIZED,
                "Link your Hackatime account again",
            )
        }
        Err(e) => {
            error!(error = %format!("{e:#}"), "couldn't fetch Hackatime projects for a submission");
            return fail(
                StatusCode::BAD_GATEWAY,
                "Couldn't check your Hackatime projects",
            );
        }
    };
    let hackatime_user = match get_hackatime_user_with_token(&hackatime_token).await {
        Ok(x) => x,
        Err(e) if is_unauthorized(&e) => {
            return fail(
                StatusCode::UNAUTHORIZED,
                "Link your Hackatime account again",
            )
        }
        Err(e) => {
            error!(error = %format!("{e:#}"), "couldn't fetch Hackatime user for a submission");
            return fail(
                StatusCode::BAD_GATEWAY,
                "Couldn't reach Hackatime, please try again",
            );
        }
    };
    let mut seconds = 0.0;
    for name in &names {
        match summaries.iter().find(|p| &p.name == name) {
            Some(project) if project.archived => {
                return fail(
                    StatusCode::BAD_REQUEST,
                    &format!("Unarchive {name} on Hackatime first"),
                )
            }
            Some(project) => seconds += project.total_duration_seconds,
            None => {
                return fail(
                    StatusCode::BAD_REQUEST,
                    &format!("Unknown Hackatime project: {name}"),
                )
            }
        }
    }
    let hours = seconds / SECONDS_PER_HOUR;
    let Some(prize_tier) = prize_tier(hours) else {
        return fail(
            StatusCode::BAD_REQUEST,
            "Not quite enough time tracked yet, make sure you reach 10 hours",
        );
    };

    let auth_data = match get_auth_data_with_token(&auth_token).await {
        Ok(x) => x,
        Err(e) if is_unauthorized(&e) => {
            return fail(
                StatusCode::UNAUTHORIZED,
                "Sign in with Hack Club Auth again",
            )
        }
        Err(e) => {
            error!(error = %format!("{e:#}"), "couldn't fetch Hack Club Auth profile for a submission");
            return fail(
                StatusCode::BAD_GATEWAY,
                "Couldn't reach Hack Club Auth, please try again",
            );
        }
    };

    let Some(_in_flight) = InFlight::claim(&auth_data.id) else {
        return fail(
            StatusCode::CONFLICT,
            "You already have a submission in progress, wait for it to finish",
        );
    };
    let earlier = match find_records::<EarlierSubmission>(
        airtable::table(),
        "HCA ID".into(),
        auth_data.id.clone(),
    )
    .await
    {
        Ok(x) => x,
        Err(e) => {
            error!(error = %format!("{e:#}"), "couldn't check earlier submissions");
            return fail(
                StatusCode::BAD_GATEWAY,
                "Couldn't save your submission, please try again",
            );
        }
    };
    let reused: Vec<&str> = names
        .iter()
        .filter(|name| {
            earlier.iter().any(|s| {
                !s.screenshot.is_empty()
                    && s.hackatime_projects
                        .lines()
                        .any(|line| line == name.as_str())
            })
        })
        .map(String::as_str)
        .collect();
    if !reused.is_empty() {
        return fail(
            StatusCode::CONFLICT,
            &format!(
                "Already used in an earlier submission: {}",
                reused.join(", ")
            ),
        );
    }
    let hackatime_projects = names.join("\n");

    let record_id = match create_unified_with_token(
        &auth_token,
        &hackatime_token,
        airtable::table(),
        repository_url,
        code_url,
        String::new(),
        description,
        names,
        ExtraFields {
            prize_tier,
            hca_id: auth_data.id,
            hackatime_projects,
            slack_id: auth_data.slack_id,
            phone_number: auth_data.phone_number,
            hackatime_id: hackatime_user.user_id.to_string(),
        },
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            let message = format!("{e:#}");
            error!(error = %message, "SUBMISSION FAILED");
            return if message.contains("not YSWS-eligible") {
                fail(
                    StatusCode::FORBIDDEN,
                    "Your Hack Club account isn't YSWS-eligible yet, check your verification at auth.hackclub.com",
                )
            } else if message.contains("No primary address") {
                fail(
                    StatusCode::BAD_REQUEST,
                    "Add a shipping address to your Hack Club account at auth.hackclub.com first",
                )
            } else if let Some((_, name)) = message.split_once("heartbeat for project: ") {
                fail(
                    StatusCode::BAD_REQUEST,
                    &format!("Hackatime doesn't have heartbeat dates for {name} yet, try again in a few minutes"),
                )
            } else {
                fail(
                    StatusCode::BAD_GATEWAY,
                    "Couldn't save your submission, please try again",
                )
            };
        }
    };

    if let Err(e) = airtable::upload_screenshot(&record_id, content_type, &filename, &bytes).await {
        error!(%record_id, error = %format!("{e:#}"), "SCREENSHOT UPLOAD FAILED, record saved without it");
        return fail(
            StatusCode::BAD_GATEWAY,
            "Your screenshot didn't upload, please submit again",
        );
    }

    info!(%record_id, hours = format!("{hours:.2}"), prize_tier, "submission created");
    HttpResponse::Ok().json(json!({ "ok": true }))
}

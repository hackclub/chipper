use actix_web::{get, http::StatusCode, HttpRequest, HttpResponse};
use hc_helper::hackatime::{
    data::{get_hackatime_project_summaries_with_token, Project},
    login::get_hackatime_token_with_handling,
};
use serde::Serialize;
use tracing::error;

use super::{fail, is_unauthorized};

#[derive(Serialize)]
struct ProjectSummary {
    name: String,
    seconds: f64,
}

fn recency(project: &Project) -> Option<&str> {
    project
        .most_recent_heartbeat
        .as_deref()
        .or(project.last_heartbeat.as_deref())
}

#[get("/hackatime/projects")]
pub async fn projects(req: HttpRequest) -> HttpResponse {
    let Some(token) = get_hackatime_token_with_handling(&req) else {
        return fail(
            StatusCode::UNAUTHORIZED,
            "Link your Hackatime account first",
        );
    };

    match get_hackatime_project_summaries_with_token(&token).await {
        Ok(mut projects) => {
            projects.sort_by(|a, b| recency(b).cmp(&recency(a)));
            HttpResponse::Ok().json(
                projects
                    .into_iter()
                    .filter(|p| !p.archived)
                    .map(|p| ProjectSummary {
                        name: p.name,
                        seconds: p.total_duration_seconds,
                    })
                    .collect::<Vec<_>>(),
            )
        }
        Err(e) if is_unauthorized(&e) => fail(
            StatusCode::UNAUTHORIZED,
            "Link your Hackatime account again",
        ),
        Err(e) => {
            error!(error = %format!("{e:#}"), "couldn't fetch Hackatime projects");
            fail(
                StatusCode::BAD_GATEWAY,
                "Couldn't load your Hackatime projects",
            )
        }
    }
}

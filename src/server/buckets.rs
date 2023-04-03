use crate::{policy::Response, Services};

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};

#[derive(serde::Deserialize)]
struct ChargeRequest {
    amount: u32,
}

async fn charge(
    Path(bucket): Path<String>,
    State(services): State<Services>,
    Json(request): Json<ChargeRequest>,
) -> impl IntoResponse {
    let result = services
        .engine
        .charge(&bucket, request.amount)
        .await
        .expect("failed to charge buckets");

    let remaining = match result.current_count {
        Some(count) => result.max_quota - count,
        None => 0,
    };

    let headers = [
        ("RateLimit-Limit", result.max_quota.to_string()),
        ("RateLimit-Remaining", remaining.to_string()),
        (
            "RateLimit-Policy",
            format!("{};w={}", result.max_quota, result.window_length_secs),
        ),
        ("RateLimit-Reset", result.window_length_secs.to_string()),
    ];

    let (code, status) = if result.is_blocked {
        (StatusCode::TOO_MANY_REQUESTS, Response::Block)
    } else if result.current_count == Some(result.max_quota) {
        (StatusCode::OK, Response::Stop)
    } else if result.current_count.unwrap_or(0) as f64 > (0.9 * result.max_quota as f64) {
        (StatusCode::OK, Response::SlowDown)
    } else {
        (StatusCode::OK, Response::Ok)
    };

    (code, headers, Json(status))
}

pub fn router(services: Services) -> Router<Services> {
    Router::new()
        .route("/:bucket/charges", post(charge))
        .with_state(services)
}

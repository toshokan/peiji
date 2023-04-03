use crate::Services;

use axum::{
    extract::{Json, Path, State},
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

    Json(result)
}

pub fn router(services: Services) -> Router<Services> {
    Router::new()
        .route("/:bucket/charges", post(charge))
        .with_state(services)
}

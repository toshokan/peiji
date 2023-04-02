use tracing::{event, Level};

use crate::{policy::Charge, Services};

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};

async fn charges(
    State(services): State<Services>,
    Json(charges): Json<Vec<Charge>>,
) -> impl IntoResponse {
    let mut ctx = match services.engine.request_ctx().await {
        Ok(ctx) => ctx,
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Err("server error"));
        }
    };

    match services.engine.charge(&mut ctx, charges).await {
        Ok(result) => return (StatusCode::OK, Ok(Json(result))),
        Err(e) => {
            event!(Level::ERROR, error = ?e);
        }
    }

    return (StatusCode::BAD_REQUEST, Err("bad request"));
}

pub async fn server(services: Services) {
    let api = Router::new()
        .route("/charges", post(charges))
        .with_state(services.clone());

    let app = Router::new()
        .nest("/api/v1", api)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let mut shutdown_receiver = services.coordinator.shutdown_receiver();

    let binding = services.config.binding;
    axum::Server::bind(&binding)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            shutdown_receiver.recv().await;
        })
        .await
        .expect("failed to serve");
}

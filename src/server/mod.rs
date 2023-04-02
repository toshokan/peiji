use std::{net::SocketAddr, sync::Arc};
use tracing::{event, Level};

use crate::{policy::Charge, Engine};

use axum::{extract::{Json, State}, Router, routing::post, http::StatusCode, response::{IntoResponse}};

async fn charges(State(engine): State<Arc<Engine>>, Json(charges): Json<Vec<Charge>>) -> impl IntoResponse {
    let mut ctx = match engine.request_ctx().await {
        Ok(ctx) => ctx,
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Err("server error"))
        }
    };
    
    match engine.charge(&mut ctx, charges).await {
        Ok(result) => {
            return (StatusCode::OK, Ok(Json(result)))
        },
        Err(e) => {
            event!(Level::ERROR, error = ?e);
        }
    }

    return (StatusCode::BAD_REQUEST, Err("bad request"));
}

pub async fn server(binding: SocketAddr, engine: Engine) {
    let engine = Arc::new(engine);

    let cleanup_engine = engine.clone();
    let _cleanup = tokio::spawn(async move { cleanup_engine.clean_up_worker().await });
    
    let api = Router::new()
        .route("/charges", post(charges))
        .with_state(Arc::clone(&engine));

    let app = Router::new()
        .nest("/api/v1", api)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    axum::Server::bind(&binding)
        .serve(app.into_make_service())
        .await
        .expect("failed to serve");
}

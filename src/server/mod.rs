use std::{net::SocketAddr, sync::Arc};
use tracing::{event, Level};

use crate::{policy::{Charge, Response}, Engine};

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
    
    let api = Router::new()
        .route("/charges", post(charges))
        .with_state(engine);

    let app = Router::new()
        .nest("/api/v1", api);

    axum::Server::bind(&binding)
        .serve(app.into_make_service())
        .await
        .expect("failed to serve");




    // let cleanup_engine = engine.clone();
    // let _cleanup = tokio::spawn(async move { cleanup_engine.clean_up_worker().await });

    // let with_env = warp::any().map(move || engine.clone());

    // let charge = warp::path!("charges")
    //     .and(warp::post())
    //     .and(warp::body::json())
    //     .and(with_env)
    //     .then(|charges: Vec<Charge>, env: Arc<Engine>| async move {
    //         let mut ctx = match env.request_ctx().await {
    //             Ok(ctx) => ctx,
    //             Err(e) => {
    //                 event!(Level::ERROR, error = ?e);
    //                 return warp::reply::with_status(
    //                     "server_error",
    //                     StatusCode::INTERNAL_SERVER_ERROR,
    //                 )
    //                 .into_response();
    //             }
    //         };

    //         match env.charge(&mut ctx, charges).await {
    //             Ok(result) => {
    //                 return json(&result).into_response();
    //             }
    //             Err(e) => {
    //                 event!(Level::ERROR, error = ?e);
    //             }
    //         }

    //         warp::reply::with_status("bad_request", StatusCode::BAD_REQUEST).into_response()
    //     });

    // let api = warp::path!("api" / "v1" / ..)
    //     .and(charge)
    //     .with(warp::log("server"));

    // warp::serve(api).run(binding).await
}

use std::sync::Arc;
use tracing::{event, Level};

use crate::{policy::Charge, Engine};

use warp::{http::StatusCode, reply::json, Filter, Reply};

pub async fn server(engine: Engine) {
    let engine = Arc::new(engine);

    let cleanup_engine = engine.clone();
    let _cleanup = tokio::spawn(async move { cleanup_engine.clean_up_worker().await });

    let with_env = warp::any().map(move || engine.clone());

    let charge = warp::path!("charges")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_env)
        .then(|charges: Vec<Charge>, env: Arc<Engine>| async move {
            let mut ctx = match env.request_ctx().await {
                Ok(ctx) => ctx,
                Err(e) => {
                    event!(Level::ERROR, error = ?e);
                    return warp::reply::with_status(
                        "server_error",
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                    .into_response();
                }
            };

            match env.charge(&mut ctx, charges).await {
                Ok(result) => {
                    return json(&result).into_response();
                }
                Err(e) => {
                    event!(Level::ERROR, error = ?e);
                }
            }

            warp::reply::with_status("bad_request", StatusCode::BAD_REQUEST).into_response()
        });

    let api = warp::path!("api" / "v1" / ..)
        .and(charge)
        .with(warp::log("server"));

    warp::serve(api).run(([0, 0, 0, 0], 80)).await
}

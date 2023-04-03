use crate::Services;

use axum::Router;

mod buckets;

pub async fn server(services: Services) {
    let api = Router::new()
        .nest("/buckets", buckets::router(services.clone()))
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

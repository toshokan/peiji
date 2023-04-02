use crate::Services;

use std::time::Duration;
use tracing::{event, Level};

#[tracing::instrument(skip_all)]
pub async fn clean_up_worker(services: Services) {
    use tokio::time::interval;

    event!(Level::DEBUG, "Starting cleanup worker");
    let mut ticker = interval(Duration::from_secs(1));

    let all = services.engine.allocations.all();

    let mut shutdown_receiver = services.coordinator.shutdown_receiver();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                event!(Level::TRACE, "Running cleanup tasks");
                services.engine.buckets.clean_up(&all).await.expect("Failed to clean up")
            },
            _ = shutdown_receiver.recv() => {
                break;
            }
        }
    }
}

use crate::Services;

use std::time::Duration;
use tracing::{event, Level};

#[tracing::instrument(skip_all)]
pub async fn clean_up_worker(services: Services) {
    use tokio::time::interval;

    event!(Level::DEBUG, "Starting cleanup worker");
    let mut ticker = interval(Duration::from_secs(1));
    let mut ctx = services
        .engine
        .request_ctx()
        .await
        .expect("Failed to get cleanup context");

    let all = ctx.allocations.all();

    let mut shutdown_receiver = services.coordinator.shutdown_receiver();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                event!(Level::TRACE, "Running cleanup tasks");
                ctx.state.clean_up(&all).await.expect("Failed to clean up")
            },
            _ = shutdown_receiver.recv() => {
                break;
            }
        }
    }
}

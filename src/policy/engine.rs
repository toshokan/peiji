use std::sync::Arc;
use tracing::{event, Level};

use crate::{policy::Response, AllocStore, BucketStore, Charge, Error};

use super::LimitView;

pub struct Engine {
    pub(crate) allocations: Arc<AllocStore>,
    pub(crate) buckets: BucketStore,
}

impl Engine {
    pub fn new(allocations: AllocStore, buckets: BucketStore) -> Self {
        Self {
            allocations: Arc::new(allocations),
            buckets,
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn charge(&self, mut charges: Vec<Charge>) -> Result<Response, Error> {
        let mut result = Response::Ok;
        let timestamp = current_timestamp();

        if charges.len() > 100 {
            event!(Level::ERROR, count = charges.len(), "Too many charges");
            Err(Error::Validation)?
        }

        let data: Vec<(Charge, LimitView<'_>)> = charges
            .drain(..)
            .flat_map(|c| {
                if let Some(limit) = self.allocations.bucket_config(&c.bucket) {
                    Some((c, limit))
                } else {
                    event!(
                        Level::WARN,
                        bucket = c.bucket.as_str(),
                        "Asked to charge unconfigured bucket"
                    );
                    None
                }
            })
            .collect();

        for (charge, _) in &data {
            if self.buckets.is_blocked(&charge.bucket).await? {
                event!(
                    Level::WARN,
                    bucket = &charge.bucket.as_str(),
                    "Refusing to charge already blocked bucket, resetting the block period."
                );
                self.buckets.block(&charge.bucket, 60).await?;
                return Ok(Response::Block);
            }

            if charge.cost > 1000 {
                event!(
                    Level::ERROR,
                    bucket = &charge.bucket.as_str(),
                    cost = charge.cost,
                    "Unreasonable cost"
                );
                Err(Error::Validation)?
            }
        }

        event!(Level::DEBUG, "Issuing charges");
        self.buckets
            .charge(data.iter().map(|(c, _)| c), timestamp)
            .await?;

        event!(Level::DEBUG, "Getting charged bucket totals");
        let counts = self.buckets.counts(data.iter().map(|(_, l)| l)).await?;

        for ((_, limit), current) in data.iter().zip(counts) {
            if current > limit.freq.raw() {
                event!(
                    Level::WARN,
                    bucket = limit.bucket,
                    limit = ?limit.freq,
                    current = current,
                    "Blocking bucket"
                );
                self.buckets.block(&limit.bucket, 5).await?;
                result = Response::Stop;
            } else if current as f64 / limit.freq.raw() as f64 > 0.9 {
                event!(
                    Level::WARN,
                    bucket = limit.bucket,
                    limit = ?limit.freq,
                    current = current,
                    "Slow down"
                );
                result = Response::SlowDown;
            }
        }

        Ok(result)
    }
}

fn current_timestamp() -> u64 {
    use std::time::SystemTime;

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    ts.try_into().unwrap_or(u64::MAX)
}

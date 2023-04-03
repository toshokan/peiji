use std::sync::Arc;
use tracing::{event, Level};

use crate::{state::ChargeResult, AllocStore, AppError, BucketStore, Error};

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

    pub async fn charge(&self, bucket: &str, amount: u32) -> Result<ChargeResult, Error> {
        let limit = self
            .allocations
            .bucket_config(bucket)
            .ok_or(AppError::UnknownBucket)?;

        if amount > 1000 {
            Err(AppError::UnreasonableCost)?;
        }

        let result = self
            .buckets
            .charge(bucket, amount, limit, std::time::SystemTime::now())
            .await?;

        Ok(result)
    }
}

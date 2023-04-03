use std::sync::Arc;
use tracing::{event, Level};

use crate::policy::block::BlockService;
use crate::{state::ChargeResult, AllocStore, AppError, BucketStore, Error};

pub struct Engine {
    pub(crate) allocations: Arc<AllocStore>,
    pub(crate) buckets: BucketStore,
    pub(crate) block: BlockService,
}

impl Engine {
    pub fn new(allocations: AllocStore, buckets: BucketStore, block: BlockService) -> Self {
        Self {
            allocations: Arc::new(allocations),
            buckets,
            block,
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

        let block_policy = self.block.policy_for(bucket);

        let result = self
            .buckets
            .charge(
                bucket,
                amount,
                limit,
                block_policy,
                std::time::SystemTime::now(),
            )
            .await?;

        Ok(result)
    }
}

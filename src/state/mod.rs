use crate::policy::{BlockPolicy, LimitView};

use deadpool_redis::{
    redis::{self, RedisError},
    PoolError,
};
use std::time::{Duration, SystemTime};
use tracing::{event, Level};

pub mod alloc;

#[derive(Debug)]
pub enum Error {
    Redis(RedisError),
    Pool(PoolError),
}
impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Self::Redis(error)
    }
}
impl From<PoolError> for Error {
    fn from(e: PoolError) -> Self {
        Self::Pool(e)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChargeResult {
    pub bucket: String,
    pub blocked_secs: Option<u32>,
    pub charge_success: bool,
    pub max_quota: u32,
    pub as_of: u64,
    pub window_start: u64,
    pub window_end: u64,
    pub window_length_secs: u32,
    pub current_count: Option<u32>,
}

#[derive(Clone)]
pub struct BucketStore {
    pool: deadpool_redis::Pool,
}

impl BucketStore {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let cfg = deadpool_redis::Config::from_url(url);
        event!(Level::TRACE, "Initializing BucketStore");

        let pool = cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("failed to create redis pool");

        let sself = Self { pool };
        sself.reload_redis_functions().await?;

        Ok(sself)
    }

    #[tracing::instrument(skip_all)]
    async fn reload_redis_functions(&self) -> Result<(), Error> {
        let module = include_str!("../../lua/peiji.lua");

        let mut conn = self.pool.get().await?;

        event!(Level::DEBUG, "loading peiji lua module");
        let result = redis::cmd("FUNCTION")
            .arg("LOAD")
            .arg("REPLACE")
            .arg(module)
            .query_async(&mut conn)
            .await?;
        event!(Level::DEBUG, "loaded peiji lua module");

        Ok(result)
    }

    pub async fn charge<'l>(
        &self,
        bucket: &str,
        amount: u32,
        limit: LimitView<'l>,
        block_policy: BlockPolicy,
        at: SystemTime,
    ) -> Result<ChargeResult, Error> {
        let mut conn = self.pool.get().await?;

        let period = limit.freq.period();
        let pts = period_start_timestamp(at, period);
        let ts = timestamp(at);

        let (is_blocked, charge_success, total, block_secs): (bool, bool, u32, u32) =
            redis::cmd("FCALL")
                .arg("charge_bucket")
                .arg(2)
                .arg(bucket) // keys[1]
                .arg(block_policy.block_key) // keys[2]
                .arg(pts) // args[1]
                .arg(limit.freq.raw()) // args[2]
                .arg(ts) // args[3]
                .arg(amount) // args[4]
                .arg(block_policy.short_timeout) // args[5]
                .arg(block_policy.long_timeout) // args[6]
                .query_async(&mut conn)
                .await?;

        let result = ChargeResult {
            bucket: bucket.to_string(),
            blocked_secs: if is_blocked { Some(block_secs) } else { None },
            charge_success,
            max_quota: limit.freq.raw(),
            as_of: ts,
            window_start: pts,
            window_end: period_end_timestamp(at, period),
            window_length_secs: period.as_secs() as u32,
            current_count: if is_blocked { None } else { Some(total) },
        };

        Ok(result)
    }

    #[tracing::instrument(skip_all)]
    pub async fn clean_up(&self, configs: &[LimitView<'_>]) -> Result<(), Error> {
        event!(Level::TRACE, "Creating pipeline");
        let mut pipe = redis::pipe();

        let now = SystemTime::now();

        for config in configs {
            let ts = period_start_timestamp(now, config.freq.period());
            event!(
                Level::TRACE,
                bucket = config.bucket,
                start = "-inf",
                end = ts,
                "Cleaning entries"
            );

            pipe.cmd("XTRIM")
                .arg(config.bucket)
                .arg("MINID")
                .arg("~")
                .arg(ts);
        }

        event!(Level::TRACE, "Executing pipeline");
        let mut conn = self.pool.get().await?;
        pipe.query_async(&mut conn).await?;

        Ok(())
    }
}

fn timestamp(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn period_end_timestamp(now: SystemTime, period: Duration) -> u64 {
    use std::ops::Add;

    timestamp(now.add(period))
}

fn period_start_timestamp(now: SystemTime, period: Duration) -> u64 {
    use std::ops::Sub;

    timestamp(now.sub(period))
}

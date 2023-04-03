use crate::policy::LimitView;

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
    pub is_blocked: bool,
    pub charge_success: bool,
    pub count: Option<CurrentCount>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrentCount {
    pub count: u32,
    pub max: u32,
    pub window_start: u64,
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
            .arg("lua")
            .arg("peiji")
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
        at: SystemTime,
    ) -> Result<ChargeResult, Error> {
        let mut conn = self.pool.get().await?;

        let pts = period_timestamp(limit.freq.period());
        let ts = timestamp(at);

        let (is_blocked, charge_success, total): (bool, bool, u32) = redis::cmd("FCALL")
            .arg("charge_bucket")
            .arg(2)
            .arg(bucket) // keys[1]
            .arg(&format!("blocked::{}", bucket)) // keys[2]
            .arg(pts) // args[1]
            .arg(limit.freq.raw()) // args[2]
            .arg(ts) // args[3]
            .arg(amount) // args[4]
            .arg(5) // args[5]
            .arg(60) // args[6]
            .query_async(&mut conn)
            .await?;

        let mut result = ChargeResult {
            bucket: bucket.to_string(),
            is_blocked,
            charge_success,
            count: None,
        };

        if !is_blocked {
            result.count = Some(CurrentCount {
                count: total,
                max: limit.freq.raw(),
                window_start: pts,
            })
        }

        Ok(result)
    }

    #[tracing::instrument(skip_all)]
    pub async fn clean_up(&self, configs: &[LimitView<'_>]) -> Result<(), Error> {
        event!(Level::TRACE, "Creating pipeline");
        let mut pipe = redis::pipe();

        for config in configs {
            let ts = period_timestamp(config.freq.period());
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

fn period_timestamp(period: Duration) -> u64 {
    use std::ops::Sub;

    let ts = SystemTime::now()
        .sub(period)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    ts.try_into().unwrap_or(u64::MAX)
}

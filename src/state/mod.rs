use crate::policy::{Charge, LimitView};

use deadpool_redis::{
    redis::{self, AsyncCommands, RedisError, Value},
    PoolError,
};
use redis::streams::StreamRangeReply;
use std::time::Duration;
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

#[derive(Debug, Clone)]
pub struct CurrentCount {
    pub bucket: String,
    pub count: u32,
    pub max: u32,
    pub window_start: u64,
}

#[derive(Clone)]
pub struct BucketStore {
    pool: deadpool_redis::Pool,
}

impl BucketStore {
    pub fn new(url: &str) -> Result<Self, Error> {
        let cfg = deadpool_redis::Config::from_url(url);
        event!(Level::TRACE, "Initializing BucketStore");

        let pool = cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("failed to create redis pool");

        Ok(Self { pool })
    }
}

impl BucketStore {
    #[tracing::instrument(skip_all, fields(key))]
    pub async fn is_blocked(&self, bucket: &str) -> Result<bool, Error> {
        let key = format!("blocked::{}", bucket);
        tracing::Span::current().record("key", &key.as_str());

        event!(Level::DEBUG, "Checking whether bucket is blocked");

        let mut conn = self.pool.get().await?;

        let result: Option<bool> = conn.get(&key).await?;
        if result.is_some() {
            event!(Level::WARN, blocked = true);
        }

        Ok(result.is_some())
    }

    #[tracing::instrument(skip_all, fields(secs = secs, key))]
    pub async fn block(&self, bucket: &str, secs: usize) -> Result<(), Error> {
        let key = format!("blocked::{}", bucket);
        tracing::Span::current().record("key", &key.as_str());

        event!(Level::WARN, "Blocking bucket");
        let mut conn = self.pool.get().await?;

        conn.set_ex(key, true, secs).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn charge(
        &self,
        charges: impl Iterator<Item = &Charge>,
        at: u64,
    ) -> Result<(), Error> {
        event!(Level::TRACE, "Creating atomic pipeline");
        let mut pipe = redis::pipe();
        pipe.atomic();

        for charge in charges {
            event!(Level::TRACE, ?charge.bucket, ?charge.cost, "requesting charge");
            pipe.xadd(
                &charge.bucket,
                at,
                &[("src", "peiji"), ("cost", &charge.cost.to_string())],
            );
        }

        let mut conn = self.pool.get().await?;

        event!(Level::TRACE, "Executing atomic pipeline");
        let _: () = pipe.query_async(&mut conn).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn counts<'l>(
        &self,
        limits: impl Iterator<Item = &'l LimitView<'l>> + Clone,
    ) -> Result<Vec<CurrentCount>, Error> {
        event!(Level::TRACE, "Creating pipeline");
        let mut pipe = redis::pipe();

        for limit in limits.clone() {
            let pts = period_timestamp(limit.freq.period());

            pipe.xrange(&limit.bucket, pts, "+");
        }

        event!(Level::TRACE, "Executing pipeline");

        let mut conn = self.pool.get().await?;
        let results: Vec<StreamRangeReply> = pipe.query_async(&mut conn).await?;
        let current_costs = results
            .iter()
            .zip(limits)
            .map(|(sr, lim)| {
                let current_count = sr
                    .ids
                    .iter()
                    .map(|ent| match ent.get("cost") {
                        Some(Value::Data(d)) => String::from_utf8(d)
                            .ok()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0),
                        _ => 0,
                    })
                    .sum();

                CurrentCount {
                    bucket: lim.bucket.to_string(),
                    count: current_count,
                    max: lim.freq.raw(),
                    window_start: period_timestamp(lim.freq.period()),
                }
            })
            .collect();

        Ok(current_costs)
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

fn period_timestamp(period: Duration) -> u64 {
    use std::ops::Sub;
    use std::time::SystemTime;

    let ts = SystemTime::now()
        .sub(period)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    ts.try_into().unwrap_or(u64::MAX)
}

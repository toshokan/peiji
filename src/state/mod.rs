use crate::policy::{Charge, LimitView};

use redis::{aio::ConnectionManager as RedisConn, AsyncCommands, Client, Pipeline, RedisError};
use std::time::Duration;
use tracing::{event, Level};

pub mod alloc;

pub struct BucketStore {
    client: Client,
}

impl BucketStore {
    pub fn new(url: &str) -> Result<Self, RedisError> {
        event!(Level::TRACE, "Initializing BucketStore");
        let client = Client::open(url)?;
        Ok(Self { client })
    }

    pub async fn ctx(&self) -> Result<StateCtx, RedisError> {
        event!(Level::TRACE, "Issuing new state context");
        Ok(StateCtx {
            conn: self.client.get_tokio_connection_manager().await?,
        })
    }
}

pub struct StateCtx {
    conn: RedisConn,
}

impl StateCtx {
    #[tracing::instrument(skip_all, fields(key))]
    pub async fn is_blocked(&mut self, bucket: &str) -> Result<bool, RedisError> {
        let key = format!("blocked::{}", bucket);
        tracing::Span::current().record("key", &key.as_str());

        event!(Level::DEBUG, "Checking whether bucket is blocked");
        let result: Option<bool> = self.conn.get(&key).await?;
        if result.is_some() {
            event!(Level::WARN, blocked = true);
        }

        Ok(result.is_some())
    }

    #[tracing::instrument(skip_all, fields(secs = secs, key))]
    pub async fn block(&mut self, bucket: &str, secs: usize) -> Result<(), RedisError> {
        let key = format!("blocked::{}", bucket);
        tracing::Span::current().record("key", &key.as_str());

        event!(Level::WARN, "Blocking bucket");
        self.conn.set_ex(key, true, secs).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn charge(
        &mut self,
        charges: impl Iterator<Item = &Charge>,
        at: u64,
    ) -> Result<(), RedisError> {
        event!(Level::TRACE, "Creating atomic pipeline");
        let mut pipe = redis::pipe();
        pipe.atomic();

        for charge in charges {
            event!(
                Level::DEBUG,
                bucket = &charge.bucket.as_str(),
                cost = &charge.cost,
                "Charging bucket"
            );
            charge_one(&mut pipe, &charge.bucket, charge.cost, at);
        }

        event!(Level::TRACE, "Executing atomic pipeline");
        let _: () = pipe.query_async(&mut self.conn).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn counts<'l>(
        &mut self,
        limits: impl Iterator<Item = &'l LimitView<'l>>,
    ) -> Result<Vec<u32>, RedisError> {
        event!(Level::TRACE, "Creating pipeline");
        let mut pipe = redis::pipe();

        for limit in limits {
            get_count(
                &mut pipe,
                limit.bucket,
                period_timestamp(limit.freq.period()),
            );
        }

        event!(Level::TRACE, "Executing pipeline");
        pipe.query_async(&mut self.conn).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn clean_up(&mut self, configs: &[LimitView<'_>]) -> Result<(), RedisError> {
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
            pipe.cmd("ZREMRANGEBYSCORE")
                .arg(&config.bucket)
                .arg("-inf")
                .arg(ts);
        }

        event!(Level::TRACE, "Executing pipeline");
        pipe.query_async(&mut self.conn).await?;

        Ok(())
    }
}

fn charge_one<'p>(
    mut pipe: &'p mut Pipeline,
    bucket: &str,
    cost: u32,
    ts: u64,
) -> &'p mut Pipeline {
    for q in 0..=cost {
        pipe = pipe.zadd(bucket, uniqueid(q), ts)
    }
    pipe
}

fn get_count<'p>(pipe: &'p mut Pipeline, bucket: &str, period: u64) -> &'p mut Pipeline {
    pipe.zcount(bucket, period, "+inf")
}

fn uniqueid(iter: u32) -> String {
    use std::time::SystemTime;

    let id = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{}", id, iter)
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

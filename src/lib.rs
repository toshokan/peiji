mod policy;
mod server;
mod state;

pub use policy::{Charge, ConfigFile, Engine, Frequency, Limit};
pub use state::alloc::AllocStore;
pub use state::BucketStore;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum Error {
    State(redis::RedisError),
    Io(std::io::Error),
    Toml(toml::de::Error),
    Validation,
}

impl From<redis::RedisError> for Error {
    fn from(e: redis::RedisError) -> Self {
        Self::State(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub binding: SocketAddr,
}

pub struct Services {
    config: Config,
    engine: Engine,
}

impl Services {
    pub fn new(config: Config, engine: Engine) -> Self {
        Self { config, engine }
    }
}

pub async fn peiji(services: Services) {
    server::server(services.config.binding, services.engine).await
}

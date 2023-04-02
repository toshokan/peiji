mod policy;
mod server;
mod state;

use std::net::SocketAddr;
pub use policy::{Charge, ConfigFile, Engine, Frequency, Limit};
pub use state::alloc::AllocStore;
pub use state::BucketStore;

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

pub async fn peiji(server_binding: SocketAddr, engine: Engine) {
    server::server(server_binding, engine).await
}

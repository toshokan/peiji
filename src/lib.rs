mod policy;
mod server;
mod state;
mod worker;

pub use policy::{Charge, ConfigFile, Engine, Frequency, Limit};
pub use state::alloc::AllocStore;
pub use state::BucketStore;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug)]
pub enum Error {
    State(state::Error),
    Io(std::io::Error),
    Toml(toml::de::Error),
    Validation,
}

impl From<state::Error> for Error {
    fn from(e: state::Error) -> Self {
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

#[derive(Debug, Clone)]
pub struct CoordinationService {
    shutdown_sender: broadcast::Sender<()>,
}

pub(crate) struct CoordinationReceiver(broadcast::Receiver<()>);

impl CoordinationReceiver {
    pub(crate) async fn recv(&mut self) {
        let _ = &self.0.recv().await.unwrap();
    }
}

impl CoordinationService {
    pub fn create_and_register() -> Self {
        let (tx, _) = broadcast::channel(1);

        {
            let tx = tx.clone();
            tokio::task::spawn(async move {
                let _ = tokio::signal::ctrl_c().await.unwrap();
                let _ = tx.send(());
            });
        }

        Self {
            shutdown_sender: tx,
        }
    }

    pub(crate) fn shutdown_receiver(&self) -> CoordinationReceiver {
        CoordinationReceiver(self.shutdown_sender.subscribe())
    }
}

#[derive(Clone)]
pub struct Services {
    config: Config,
    engine: Arc<Engine>,
    coordinator: CoordinationService,
}

impl Services {
    pub fn new(config: Config, engine: Engine) -> Self {
        let coordinator = CoordinationService::create_and_register();

        Self {
            config,
            engine: Arc::new(engine),
            coordinator,
        }
    }
}

pub async fn peiji(services: Services) {
    let _ = {
        let services = services.clone();
        tokio::spawn(async move { crate::worker::clean_up_worker(services).await })
    };

    server::server(services).await
}

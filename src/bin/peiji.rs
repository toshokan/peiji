use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use peiji::ConfigFile;

#[derive(Debug)]
struct Config {
    redis_uri: String,
    config_file: String,
    listen_ip: String,
    listen_port: u16,
}

impl Config {
    fn from_env() -> Self {
        Self {
            redis_uri: std::env::var("REDIS_URI").expect("Supply REDIS_URI"),
            config_file: std::env::var("CONFIG_FILE_PATH").expect("Supply CONFIG_FILE_PATH"),
            listen_ip: std::env::var("LISTEN_IP").expect("Supply LISTEN_IP"),
            listen_port: std::env::var("LISTEN_PORT")
                .expect("Supply LISTEN_PORT")
                .parse()
                .expect("invalid LISTEN_PORT"),
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let config_file = ConfigFile::from_file(&config.config_file).expect("Bad config file");

    let alloc = peiji::AllocStore::new(config_file.limits);
    let state = peiji::BucketStore::new(&config.redis_uri).expect("Failed to initialize store");

    let engine = peiji::Engine::new(alloc, state);

    let ip = IpAddr::from_str(&config.listen_ip).expect("invalid listen ip");

    let binding = SocketAddr::new(ip, config.listen_port);

    let server_config = peiji::Config { binding };

    let services = peiji::Services::new(server_config, engine);

    peiji::peiji(services).await
}

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use peiji::ConfigFile;

#[derive(Debug)]
struct Config {
    redis_uri: String,
    config_file: String,
    listen_ip: String,
    listen_port: u16,
    short_block_timeout_secs: u32,
    long_block_timeout_secs: u32,
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
            short_block_timeout_secs: std::env::var("SHORT_BLOCK_TIMEOUT_SECS")
                .expect("supply SHORT_BLOCK_TIMEOUT_SECS")
                .parse()
                .expect("invalue SHORT_BLOCK_TIMEOUT_SECS"),
            long_block_timeout_secs: std::env::var("LONG_BLOCK_TIMEOUT_SECS")
                .expect("supply LONG_BLOCK_TIMEOUT_SECS")
                .parse()
                .expect("invalue LONG_BLOCK_TIMEOUT_SECS"),
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let config_file = ConfigFile::from_file(&config.config_file).expect("Bad config file");

    let ip = IpAddr::from_str(&config.listen_ip).expect("invalid listen ip");

    let binding = SocketAddr::new(ip, config.listen_port);

    let server_config = peiji::Config {
        binding,
        short_block_timeout_secs: config.short_block_timeout_secs,
        long_block_timeout_secs: config.long_block_timeout_secs,
    };

    let alloc = peiji::AllocStore::new(config_file.limits);
    let state = peiji::BucketStore::new(&config.redis_uri)
        .await
        .expect("Failed to initialize store");
    let block = peiji::BlockService::new(server_config.clone());

    let engine = peiji::Engine::new(alloc, state, block);

    let services = peiji::Services::new(server_config, engine);

    peiji::peiji(services).await
}

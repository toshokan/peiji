use peiji::ConfigFile;


#[derive(Debug)]
struct Config {
    redis_uri: String,
    config_file: String
}

impl Config {
    fn from_env() -> Self {
	Self {
	    redis_uri: std::env::var("REDIS_URI")
		.expect("Supply REDIS_URI"),
	    config_file: std::env::var("CONFIG_FILE_PATH")
		.expect("Supply CONFIG_FILE_PATH")
	}
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let config_file = ConfigFile::from_file(&config.config_file)
	.expect("Bad config file");

    let alloc = peiji::AllocStore::new(config_file.limits);
    let state = peiji::BucketStore::new(&config.redis_uri)
	.expect("Failed to initialize store");
    
    let engine = peiji::Engine::new(alloc, state);

    peiji::peiji(engine).await
}

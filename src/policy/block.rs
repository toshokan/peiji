use crate::Config;

#[derive(Debug, Clone)]
pub struct BlockPolicy {
    pub block_key: String,
    pub short_timeout: u32,
    pub long_timeout: u32,
}

pub struct BlockService {
    config: Config,
}

impl BlockService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn policy_for(&self, bucket: &str) -> BlockPolicy {
        BlockPolicy {
            block_key: format!("blocked::{}", bucket),
            short_timeout: self.config.short_block_timeout_secs,
            long_timeout: self.config.long_block_timeout_secs,
        }
    }
}

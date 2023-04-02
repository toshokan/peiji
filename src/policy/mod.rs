mod engine;

use crate::Error;
pub use engine::Engine;

use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Charge {
    pub bucket: String,
    pub cost: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct Limit {
    pub bucket: String,
    #[serde(flatten)]
    pub freq: Frequency,
}

#[derive(Debug)]
pub struct LimitView<'a> {
    pub bucket: &'a str,
    pub freq: Frequency,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", content = "freq")]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Secondly(u32),
    Minutely(u32),
    Hourly(u32),
    Daily(u32),
    Weekly(u32),
    Monthly(u32),
}

impl Default for Frequency {
    fn default() -> Self {
        Self::Secondly(0)
    }
}

impl Frequency {
    pub fn period(&self) -> Duration {
        match self {
            Self::Secondly(_) => Duration::from_secs(1),
            Self::Minutely(_) => Duration::from_secs(60),
            Self::Hourly(_) => Duration::from_secs(60 * 60),
            Self::Daily(_) => Duration::from_secs(60 * 60 * 24),
            Self::Weekly(_) => Duration::from_secs(60 * 60 * 24 * 7),
            Self::Monthly(_) => Duration::from_secs(60 * 60 * 24 * 30),
        }
    }

    pub fn raw(&self) -> u32 {
        match self {
            Self::Secondly(c)
            | Self::Minutely(c)
            | Self::Hourly(c)
            | Self::Daily(c)
            | Self::Weekly(c)
            | Self::Monthly(c) => *c,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
    Ok,
    SlowDown,
    Stop,
    Block,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfigFile {
    pub limits: Vec<Limit>,
}

impl ConfigFile {
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(&path)?;

        Ok(toml::from_str(&contents)?)
    }
}

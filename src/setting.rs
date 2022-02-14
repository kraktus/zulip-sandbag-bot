// based on https://github.com/mehcode/config-rs/blob/0.11.0/examples/hierarchical-env/src/settings.rs

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::time::Duration;
use serde_with::{serde_as, DurationSeconds};

use crate::zulip::ZulipConfig;

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "as_true")]
    pub debug: bool,
    pub zulip: ZulipConfig,
    pub lichess_token: Option<String>,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub sleep_time: Duration,
}

fn as_true() -> bool {
    true
}


impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        // Start off by merging in the "default" configuration file
        s.merge(File::with_name("config/base"))?;
        // Add in a prod configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/prod").required(false))?;
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("app"))?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.deserialize()
    }
}

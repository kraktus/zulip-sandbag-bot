// based on https://github.com/mehcode/config-rs/tree/master/examples/hierarchical-env

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Api {
    // zulip
    email: String,
    key: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    debug: bool,
    zulip: Api,
    lichess_token: String,
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

        // Now that we're done, let's access our configuration
        println!("debug: {:?}", s.get_bool("debug"));
        println!("database: {:?}", s.get::<String>("database.url"));

        // You can deserialize (and thus freeze) the entire configuration as
        s.deserialize()
    }
}

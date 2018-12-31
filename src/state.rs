use super::Config;
use std::{env, path};

#[derive(Debug, Clone)]
pub struct PuizcloudState {
    config: Config,
    full_data_path: path::PathBuf,
}

impl PuizcloudState {
    pub fn new(config: Config) -> PuizcloudState {
        let full_data_path = if config.data_path().is_relative() {
            env::current_dir()
                .expect("Unable to determine current directory")
                .join(config.data_path())
        } else {
            config.data_path().to_owned()
        };
        PuizcloudState {
            config,
            full_data_path,
        }
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn full_data_path(&self) -> &path::Path {
        &self.full_data_path
    }
}

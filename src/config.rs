use std::default::Default;
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Config {
    pub ip: String,
    pub port: u64,
    pub data: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ip: "127.0.0.1".into(),
            port: 8080,
            data: "data/".into(),
        }
    }
}

impl Config {
    pub fn data_path(&self) -> &Path {
        Path::new(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    // test that the default config delivered is indeeed the same as Config::default()
    #[test]
    fn test_default_data() {
        let default_config_toml = include_str!("../puizcloud.toml");
        let config_from_toml: Config =
            toml::from_str(&default_config_toml).expect("Failed to parse TOML");
        let default_config = Config::default();
        assert_eq!(&default_config, &config_from_toml);
    }
}

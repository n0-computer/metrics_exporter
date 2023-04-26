use std::fs;
use std::{fmt, path::PathBuf};
use anyhow::Error;
use serde::{Serialize, Deserialize};
use names::Generator;

use std::fs::File;
use std::io::Write;
use std::io::Read;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub scrape_addr: String,
    pub push_addr: String,
    pub instance: String,
    pub job: String,
    pub scrape_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        let mut name_generator = Generator::default();
        Self {
            scrape_addr: "http://127.0.0.1:9090".to_string(),
            push_addr: "".to_string(),
            instance: format!("devbox-{}", name_generator.next().unwrap()),
            job: "metrics_exporter".to_string(),
            scrape_interval: 15,
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "name: {}\nscrape_address: {}\npush_address: {}\njob_name: {}\nscrape_interval: {}",
            self.instance, self.scrape_addr, self.push_addr, self.job, self.scrape_interval
        )
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_args(
        scrape_addr: Option<String>,
        push_addr: Option<String>,
        instance: Option<String>,
        job: Option<String>,
        push_interval: Option<u64>,
    ) -> Self {
        let mut cfg = Self::default();
        cfg.scrape_addr = scrape_addr.unwrap_or(cfg.scrape_addr);
        cfg.push_addr = push_addr.unwrap_or(cfg.push_addr);
        cfg.instance = instance.unwrap_or(cfg.instance);
        cfg.job = job.unwrap_or(cfg.job);
        cfg.scrape_interval = push_interval.unwrap_or(cfg.scrape_interval);
        cfg
    }

    pub fn new_from_cfg(cfg_path: PathBuf) -> Result<Self, Error> {
        if fs::metadata(&cfg_path).is_ok() {
            let mut cfg_file = File::open(cfg_path)?;
            let mut cfg_toml = String::new();
            cfg_file.read_to_string(&mut cfg_toml)?;

            let cfg: Config = toml::from_str(&cfg_toml)?;
            Ok(cfg)
        } else {
            let cfg = Config::new();
            let mut cfg_file = File::create(cfg_path)?;
            let cfg_toml = toml::to_string(&cfg)?;
            cfg_file.write_all(cfg_toml.as_bytes())?;
            Ok(cfg)
        }
    }
}

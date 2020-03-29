use anyhow::{bail, Result};
use serde::Deserialize;
use serde_yaml;
use std::{collections::HashMap, fs::File, path::PathBuf};

fn config_file() -> PathBuf {
    std::env::var("FERN_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut home = dirs::home_dir().unwrap();
            home.push(".fern.config.yaml");
            home
        })
}

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    seeds: HashMap<String, serde_yaml::Value>,
}

fn load(p: PathBuf) -> Config {
    serde_yaml::from_reader(File::open(p).unwrap()).unwrap()
}

pub fn folder(lang: String) -> Result<()> {
    let config = config_file();

    if !config.exists() {
        bail!("Config file at {:?} does not exist", config)
    }

    let config = load(config);

    if let Some(yaml) = config.seeds.get(&lang) {
        let f = File::create("fern.yaml").unwrap();
        serde_yaml::to_writer(f, yaml).expect("this to work");
        println!("Created new fern.yaml file for rust");
        Ok(())
    } else {
        bail!("Did not find {} in config", lang)
    }
}

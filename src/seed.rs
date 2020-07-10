use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::PathBuf};

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    seeds: HashMap<String, serde_yaml::Value>,
}

pub fn folder(lang: Option<String>) -> Result<()> {
    let language = if let Some(lang) = lang {
        lang
    } else {
        bail!("No langauge defiend to seed the fern.yaml file with.")
    };

    let config = config_file();

    if !config.exists() {
        bail!("Config file at {:?} does not exist", config)
    }

    let config = load(config)?;

    if let Some(yaml) = config.seeds.get(&language) {
        let f = File::create("fern.yaml")?;
        serde_yaml::to_writer(f, yaml).with_context(|| "unable to write fern.yaml".to_string())?;
        println!("Created new fern.yaml file for rust");
        Ok(())
    } else {
        bail!("Did not find {} in config", language)
    }
}

fn load(p: PathBuf) -> Result<Config> {
    let f = File::open(p.clone())?;
    serde_yaml::from_reader(f).with_context(|| format!("Unable to read configuration {:?}", p))
}

fn config_file() -> PathBuf {
    std::env::var("FERN_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut home = dirs::home_dir().expect("Unable to get the the users 'home' directory");
            home.push(".fern.config.yaml");
            home
        })
}

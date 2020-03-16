use core::fmt;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::fs::File;

#[derive(Debug, Eq, PartialEq)]
struct Steps {
    pub values: Vec<String>,
}

impl Default for Steps {
    fn default() -> Self {
        Steps { values: Vec::new() }
    }
}

impl<'de> Deserialize<'de> for Steps {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OneOrManyVisitor {}

        impl<'de> Visitor<'de> for OneOrManyVisitor {
            type Value = Steps;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("either \"auto\" or a port number")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Steps {
                    values: vec![value.to_owned()],
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();

                while let Ok(Some(val)) = seq.next_element::<String>() {
                    values.push(val)
                }

                Ok(Steps { values })
            }
        }

        deserializer.deserialize_any(OneOrManyVisitor {})
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct FolderConfig {
    #[serde(default)]
    fmt: Steps,
    #[serde(default)]
    build: Steps,
    #[serde(default)]
    test: Steps,
    #[serde(default)]
    check: Steps,
}

use clap::{App, SubCommand};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(steps: Steps, cwd: &Path) {
    for value in steps.values {
        let output = Command::new("sh")
            .arg("-c")
            .arg(value.clone())
            .current_dir(cwd)
            .output()
            .expect("unable to run command");

        println!("{}", String::from_utf8_lossy(&output.stdout[..]));
    }
}

fn find_all_leafs() -> Vec<PathBuf> {
    let mut fern_leaves = Vec::new();
    for result in WalkBuilder::new("./").build() {
        let entry = result.unwrap();
        if entry.metadata().unwrap().is_dir() {
            continue;
        }

        if entry.path().file_name().unwrap() != "fern.yaml" {
            continue;
        }

        fern_leaves.push(entry.into_path());
    }

    fern_leaves
}

fn main() {
    let matches = App::new("fern")
        .subcommand(SubCommand::with_name("fmt").about("running any formatting"))
        .subcommand(SubCommand::with_name("build").about("running any building"))
        .subcommand(SubCommand::with_name("test").about("running any testing"))
        .subcommand(SubCommand::with_name("check").about("running any checking"))
        .get_matches();

    let fern_leafs = find_all_leafs();

    for leaf in fern_leafs {
        let file = File::open(leaf.clone()).unwrap();
        let working_dir = leaf.parent().unwrap();
        let config: FolderConfig = serde_yaml::from_reader(file).unwrap();

        let steps = match matches.subcommand_name() {
            Some("fmt") => config.fmt,
            Some("build") => config.build,
            Some("test") => config.test,
            Some("check") => config.check,
            _ => Steps::default(),
        };

        run(steps, working_dir);
    }
}

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
                formatter.write_str("either single string or sequence of strings")
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
        Command::new("sh")
            .arg("-c")
            .arg(value.clone())
            .current_dir(cwd)
            .spawn()
            .expect("unable to run command")
            .wait()
            .expect("child process was not successful");
    }
}

fn find_all_leaves() -> Vec<PathBuf> {
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
        .subcommand(SubCommand::with_name("leaves").about("list all leaves fern will consider"))
        .get_matches();

    let fern_leaves = find_all_leaves();

    if matches.is_present("leaves") {
        println!("Considering leaves:");
        for leaf in fern_leaves {
            println!(" *\t{}", leaf.to_string_lossy())
        }
        return;
    }

    for leaf in fern_leaves {
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

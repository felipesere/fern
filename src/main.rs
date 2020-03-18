use core::fmt;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::fs::File;

use git_version::git_version;
const GIT_VERSION: &str = git_version!();
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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

use clap::{App, Arg, ArgMatches, SubCommand};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let version = format!("{} ({})", VERSION, GIT_VERSION);
    let mut app = App::new("fern").version(&*version);
    let commands = vec![
        ("fmt", "running any formatting"),
        ("build", "running any building"),
        ("test", "running any testing"),
        ("check", "doing any checks"),
    ];

    let here = SubCommand::with_name("here")
        .about("Only look at the current directory for fern.yaml files.");

    for (command, about) in commands {
        app = app.subcommand(
            SubCommand::with_name(command)
                .about(about)
                .subcommand(here.clone()),
        );
    }

    app = app.subcommand(
        SubCommand::with_name("leaves")
            .about("list all leaves fern will consider")
            .arg(
                Arg::with_name("porcelain")
                    .short("p")
                    .long("porcelain")
                    .required(false),
            ),
    );

    let matches = app.get_matches();

    match matches.subcommand() {
        ("leaves", extra_args) => print_leaves(is_present(extra_args, "porcelain")),
        (command, extra_args) => run_leaves(command, is_present(extra_args, "here")),
    }
}

fn is_present(extra: Option<&ArgMatches>, name: &'static str) -> bool {
    extra.map(|args| args.is_present(name)).unwrap_or(false)
}

fn run_leaves(command: &str, here: bool) {
    if here {
        run_single_leaf(PathBuf::from("./fern.yaml"), command)
    } else {
        for leaf in find_all_leaves() {
            run_single_leaf(leaf, command)
        }
    }
}

fn run_single_leaf(leaf: PathBuf, command: &str) {
    let file = File::open(leaf.clone()).unwrap();
    let working_dir = leaf.parent().unwrap();
    let config: FolderConfig = serde_yaml::from_reader(file).unwrap();

    let steps = match command {
        "fmt" => config.fmt,
        "build" => config.build,
        "test" => config.test,
        "check" => config.check,
        _ => Steps::default(),
    };

    run_all_steps(steps, working_dir);
}

fn run_all_steps(steps: Steps, cwd: &Path) {
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

fn print_leaves(porcelain: bool) {
    let fern_leaves = find_all_leaves();
    if porcelain {
        println!(
            "{}",
            fern_leaves
                .iter()
                .map(|s| s.to_string_lossy().to_owned())
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        println!("Considering leaves:");
        for leaf in fern_leaves {
            println!(" *\t{}", leaf.to_string_lossy())
        }
    }
}

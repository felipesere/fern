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

#[derive(Eq, PartialEq)]
enum Depth {
    Here,
    Recursive,
}

enum PrintStyle {
    Pretty,
    Porcelain,
}

enum Commands {
    Fmt(Depth),
    Build(Depth),
    Test(Depth),
    Check(Depth),
}

impl Commands {
    fn pick(&self, folder: FolderConfig) -> Steps {
        match self {
            Commands::Fmt(_) => folder.fmt,
            Commands::Build(_) => folder.build,
            Commands::Test(_) => folder.test,
            Commands::Check(_) => folder.check,
        }
    }

    fn is_recursive(&self) -> bool {
        match self {
            Commands::Fmt(depth)
            | Commands::Build(depth)
            | Commands::Test(depth)
            | Commands::Check(depth) => *depth == Depth::Recursive,
        }
    }
}

enum Options {
    Exec(Commands),
    Leaves(PrintStyle),
    Help,
    Version,
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

fn command() -> Options {
    let mut args = pico_args::Arguments::from_env();
    let mut depth = Depth::Recursive;
    if args.contains("here") {
        depth = Depth::Here;
    }

    if args.contains("fmt") {
        return Options::Exec(Commands::Fmt(depth));
    }

    if args.contains("build") {
        return Options::Exec(Commands::Build(depth));
    }

    if args.contains("check") {
        return Options::Exec(Commands::Check(depth));
    }

    if args.contains("test") {
        return Options::Exec(Commands::Test(depth));
    }

    if args.contains("leaves") {
        if args.contains(["-p", "--porcelain"]) {
            return Options::Leaves(PrintStyle::Porcelain);
        } else {
            return Options::Leaves(PrintStyle::Pretty);
        }
    }

    if args.contains(["-v", "--version"]) {
        return Options::Version;
    }

    Options::Help
}

fn main() {
    let version = format!("{} ({})", VERSION, GIT_VERSION);

    match command() {
        Options::Version => println!("fern version {}", version),
        Options::Leaves(style) => print_leaves(style),
        Options::Help => println!("help"),
        Options::Exec(c) => run_leaves(c),
    }
}

fn run_leaves(command: Commands) {
    if command.is_recursive() {
        for leaf in find_all_leaves() {
            run_single_leaf(leaf, &command)
        }
    } else {
        run_single_leaf(PathBuf::from("./fern.yaml"), &command)
    }
}

fn run_single_leaf(leaf: PathBuf, command: &Commands) {
    let file = File::open(leaf.clone()).unwrap();
    let working_dir = leaf.parent().unwrap();
    let config: FolderConfig = serde_yaml::from_reader(file).unwrap();

    let steps = command.pick(config);

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

fn print_leaves(style: PrintStyle) {
    let fern_leaves = find_all_leaves();
    match style {
        PrintStyle::Porcelain => println!(
            "{}",
            fern_leaves
                .iter()
                .map(|s| s.to_string_lossy().to_owned())
                .collect::<Vec<_>>()
                .join(" ")
        ),
        PrintStyle::Pretty => {
            println!("Considering leaves:");
            for leaf in fern_leaves {
                println!(" *\t{}", leaf.to_string_lossy())
            }
        }
    };
}

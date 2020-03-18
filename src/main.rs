use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use git_version::git_version;
use ignore::WalkBuilder;
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

use steps::Steps;

mod steps;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Did not find a fern.yaml file in here"))]
    NoLeafFound,

    #[snafu(display("Could not read file at {}: {}", file.to_string_lossy(), source))]
    CouldNotReadFile {
        file: PathBuf,
        source: serde_yaml::Error,
    },
    #[snafu(display("Did not find {}: {}", command, source))]
    DidNotFindCommand {
        command: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to execute command '{}': {}", command, source))]
    FailedToExecuteCommand {
        command: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to execute command '{}': exit code {}", command, exit_code))]
    CommandDidNotSucceed { command: String, exit_code: i32 },
}

type Result<T, E = Error> = std::result::Result<T, E>;

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

const GIT_VERSION: &str = git_version!();
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let version = format!("{} ({})", VERSION, GIT_VERSION);

    let res = match command() {
        Options::Version => print_version(version),
        Options::Leaves(style) => print_leaves(style),
        Options::Help => print_help(),
        Options::Exec(c) => run_leaves(c),
    };

    if let Result::Err(e) = res {
        println!("{}", e)
    }
}

fn print_version(version: String) -> Result<()> {
    println!("fern version {}", version);
    Result::Ok(())
}

fn print_help() -> Result<()> {
    println!("TODO: help");
    Result::Ok(())
}

fn run_leaves(command: Commands) -> Result<()> {
    if command.is_recursive() {
        for leaf in find_all_leaves() {
            run_single_leaf(leaf, &command)?
        }
        return Ok(());
    } else {
        run_single_leaf(PathBuf::from("./fern.yaml"), &command)
    }
}

fn run_single_leaf(leaf: PathBuf, command: &Commands) -> Result<()> {
    if !leaf.exists() {
        return Result::Err(Error::NoLeafFound);
    }

    let file = File::open(leaf.clone()).unwrap();

    let working_dir = leaf.parent().unwrap();
    let config: FolderConfig =
        serde_yaml::from_reader(file).context(CouldNotReadFile { file: leaf.clone() })?;

    let steps = command.pick(config);

    run_all_steps(steps, working_dir)
}

// TODO error handling
fn run_all_steps(steps: Steps, cwd: &Path) -> Result<()> {
    for value in steps.values {
        let ecode = Command::new("sh")
            .arg("-c")
            .arg(value.clone())
            .current_dir(cwd)
            .spawn()
            .context(DidNotFindCommand {
                command: value.clone(),
            })?
            .wait()
            .context(FailedToExecuteCommand {
                command: value.clone(),
            })?;
        if !ecode.success() {
            return Result::Err(Error::CommandDidNotSucceed {
                command: value.clone(),
                exit_code: ecode.code().unwrap_or(-1),
            });
        }
    }

    Result::Ok(())
}

fn print_leaves(style: PrintStyle) -> Result<()> {
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

    Ok(())
}

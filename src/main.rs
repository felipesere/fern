use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use git_version::git_version;
use ignore::WalkBuilder;
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

use pico_args::Arguments;
use steps::Steps;

mod steps;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Did not find a fern.yaml file in here"))]
    NoLeafFoundHere,

    #[snafu(display("Did not find any fern.yaml files"))]
    NoLeafFoundAnywhere,

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

    #[snafu(display("Failed to execute command '{}': exit code {}", command, exit_code))]
    CommandDidNotSucceed { command: String, exit_code: i32 },
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Eq, PartialEq, Copy, Clone)]
enum Depth {
    Here,
    Recursive,
}

enum PrintStyle {
    Pretty,
    Porcelain,
}

enum Commands {
    Fmt,
    Build,
    Test,
    Check,
}

impl Commands {
    fn pick(&self, folder: FolderConfig) -> Steps {
        match self {
            Commands::Fmt => folder.fmt,
            Commands::Build => folder.build,
            Commands::Test => folder.test,
            Commands::Check => folder.check,
        }
    }
}

enum Options {
    Exec(Commands, ExecOptions),
    Leaves(PrintStyle),
    Help,
    Version,
}

#[derive(Clone, Copy)]
struct ExecOptions {
    depth: Depth,
    quiet: bool,
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

fn option(c: Commands, mut args: Arguments) -> Options {
    let depth = if let Some("here") = args.subcommand().ok().flatten().as_deref() {
        Depth::Here
    } else {
        Depth::Recursive
    };

    let quiet = args.contains(["-q", "--quiet"]);

    Options::Exec(c, ExecOptions { depth, quiet })
}

fn command() -> Options {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-v", "--version"]) {
        return Options::Version;
    }

    if let Ok(Some(cmd)) = args.subcommand() {
        match cmd.as_str() {
            "fmt" => return option(Commands::Fmt, args),
            "build" => return option(Commands::Build, args),
            "check" => return option(Commands::Check, args),
            "test" => return option(Commands::Test, args),
            "leaves" => {
                if args.contains(["-p", "--porcelain"]) {
                    return Options::Leaves(PrintStyle::Porcelain);
                } else {
                    return Options::Leaves(PrintStyle::Pretty);
                }
            }
            _ => {}
        }
    }
    Options::Help
}

const GIT_VERSION: &str = git_version!();
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let res = match command() {
        Options::Version => print_version(),
        Options::Help => print_help(),
        Options::Leaves(style) => print_leaves(style),
        Options::Exec(command, opts) => {
            let res = run_leaves(command, opts);
            if opts.quiet {
                Ok(())
            } else {
                res
            }
        }
    };

    if let Result::Err(e) = res {
        println!("{}", e)
    }
}

fn print_version() -> Result<()> {
    let version = format!("{} ({})", VERSION, GIT_VERSION);
    println!("fern version {}", version);
    Ok(())
}

fn print_help() -> Result<()> {
    println!(
        r#"
    fern
    Gives different parts of your mono-repo a unified interface to run certain tasks.

    USAGE:
        fern [FLAGS] [SUBCOMMAND] [OPTIONS]

    FLAGS:
        -v, --version    Prints version information

    SUBCOMMANDS:
        leaves      for showing all fern.yaml files. Has a -p | --porcelain option for better tooling
        fmt         for anything formatting related
        build       for anything related to building the app
        test        for running any kind of tests
        check       for things like type-checks or build-checks

    [OPTIONS]
        here        to only look in the current dir for a fern.yaml file, not recurisively searching the entire tree 
        -q | --quiet  to silence errors when no fern.yaml file is present


    Examples
        $: fern fmt  # will look for all fern.yaml files and run the 'fmt' target
        $: fern fmt here  # will look only use the one in the current directory

    Any other input will print this help menu.
   "#
    );
    Ok(())
}

fn run_leaves(command: Commands, opts: ExecOptions) -> Result<()> {
    if opts.depth == Depth::Recursive {
        let leaves = find_all_leaves();
        if leaves.is_empty() {
            Result::Err(Error::NoLeafFoundAnywhere)
        } else {
            for leaf in leaves {
                run_single_leaf(leaf, &command)?;
            }
            Ok(())
        }
    } else {
        run_single_leaf(PathBuf::from("./fern.yaml"), &command)
    }
}

fn run_single_leaf(leaf: PathBuf, command: &Commands) -> Result<()> {
    if !leaf.exists() {
        return Result::Err(Error::NoLeafFoundHere);
    }

    let file = File::open(leaf.clone()).unwrap();

    let working_dir = leaf.parent().unwrap();
    let config: FolderConfig =
        serde_yaml::from_reader(file).context(CouldNotReadFile { file: leaf.clone() })?;

    let steps = command.pick(config);

    run_all_steps(steps, working_dir)
}

fn run_all_steps(steps: Steps, cwd: &Path) -> Result<()> {
    for value in steps.values {
        let ecode = Command::new("sh")
            .arg("-c")
            .arg(value.clone())
            .current_dir(cwd)
            .status()
            .context(DidNotFindCommand {
                command: value.clone(),
            })?;
        if !ecode.success() {
            return Result::Err(Error::CommandDidNotSucceed {
                command: value,
                exit_code: ecode.code().unwrap_or(-1),
            });
        }
    }

    Ok(())
}

fn print_leaves(style: PrintStyle) -> Result<()> {
    let fern_leaves = find_all_leaves();
    match style {
        PrintStyle::Porcelain => println!(
            "{}",
            fern_leaves
                .iter()
                .map(|s| s.to_string_lossy())
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

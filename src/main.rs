use std::fs::File;
use std::path::{Path, PathBuf};
use std::{
    collections::HashMap,
    process::{self, Command},
};

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

    #[snafu(display("There was an error when reading the file: {}", source))]
    FailedToReadFernFile { source: serde_yaml::Error },

    #[snafu(display("Did not find {}: {}", command, source))]
    DidNotFindCommand {
        command: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to execute command '{}': exit code {}", command, exit_code))]
    CommandDidNotSucceed { command: String, exit_code: i32 },

    #[snafu(display("No langauge defiend to seed the fern.yaml file with."))]
    NoLanguageDefined,

    #[snafu(display("Config file at {:?} does not exist", location))]
    ConfigDoesNotExist { location: PathBuf },

    #[snafu(display("Did not find {} in config", language))]
    DidNotFindLanguageForSeedfile { language: String },
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
    Seed { language: Option<String> },
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

impl FolderConfig {
    fn from_yaml<R: std::io::Read>(source: R) -> Result<Self> {
        serde_yaml::from_reader(source).context(FailedToReadFernFile {})
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

fn opts(mut args: Arguments) -> ExecOptions {
    let depth = if let Some("here") = args.subcommand().ok().flatten().as_deref() {
        Depth::Here
    } else {
        Depth::Recursive
    };

    let quiet = args.contains(["-q", "--quiet"]);
    ExecOptions { depth, quiet }
}

fn command() -> Options {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-v", "--version"]) {
        return Options::Version;
    }

    if let Ok(Some(cmd)) = args.subcommand() {
        match cmd.as_str() {
            "fmt" => return Options::Exec(Commands::Fmt, opts(args)),
            "build" => return Options::Exec(Commands::Build, opts(args)),
            "check" => return Options::Exec(Commands::Check, opts(args)),
            "test" => return Options::Exec(Commands::Test, opts(args)),
            "leaves" => {
                if args.contains(["-p", "--porcelain"]) {
                    return Options::Leaves(PrintStyle::Porcelain);
                } else {
                    return Options::Leaves(PrintStyle::Pretty);
                }
            }
            "seed" => {
                let language = args.subcommand().ok().flatten();
                return Options::Seed { language };
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
        Options::Seed { language } => {
            if let Some(lang) = language {
                seed_folder(lang)
            } else {
                Err(Error::NoLanguageDefined)
            }
        }
    };

    if let Result::Err(e) = res {
        println!("{}", e);
        process::exit(-1);
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
    let config = FolderConfig::from_yaml(file)?;

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

fn seed_folder(lang: String) -> Result<()> {
    let config = config_file();

    if !config.exists() {
        return Err(Error::ConfigDoesNotExist { location: config });
    }

    let config = load(config);

    if let Some(yaml) = config.seeds.get(&lang) {
        let f = File::create("fern.yaml").unwrap();
        serde_yaml::to_writer(f, yaml).expect("this to work");
        println!("Created new fern.yaml file for rust");
        Ok(())
    } else {
        Err(Error::DidNotFindLanguageForSeedfile { language: lang })
    }
}

#[cfg(test)]
mod tests {
    use crate::FolderConfig;

    #[test]
    fn it_parses_a_correct_yaml_file() {
        let yaml = r#"
       fmt: Something
       build: Fancy
       "#;

        let folder = FolderConfig::from_yaml(yaml.as_bytes()).unwrap();

        assert_eq!(folder.fmt.values, vec![String::from("Something")]);
        assert_eq!(folder.build.values, vec![String::from("Fancy")]);
    }

    #[test]
    fn it_ignores_unknown_fields() {
        let yaml = r#"
       fmt: Something
       not: Important
       something: 12
       "#;

        let folder = FolderConfig::from_yaml(yaml.as_bytes()).unwrap();

        assert_eq!(folder.fmt.values, vec![String::from("Something")]);
    }

    #[test]
    fn it_reports_adequate_errors() {
        let yaml = r#"fmt: Something
        has no value:
       "#;

        let error = FolderConfig::from_yaml(yaml.as_bytes())
            .unwrap_err()
            .to_string();

        assert_eq!("There was an error when reading the file: mapping values are not allowed in this context at line 2 column 21", error)
    }

    #[test]
    fn it_reports_errors_for_known_keys() {
        let yaml = "fmt: 12";

        let error = FolderConfig::from_yaml(yaml.as_bytes())
            .unwrap_err()
            .to_string();

        assert_eq!("There was an error when reading the file: fmt: invalid type: integer `12`, expected either single string or sequence of strings at line 1 column 6", error)
    }
}

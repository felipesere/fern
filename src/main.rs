use std::fs::File;
use std::path::{Path, PathBuf};
use std::{
    collections::{HashMap, HashSet},
    process::{self, Command},
};

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

pub(crate) struct Operation(String);

enum Options {
    Exec(Operation, ExecOptions),
    Seed { language: Option<String> },
    Leaves(PrintStyle),
    Help,
    Version,
    List(PrintStyle),
}

#[derive(Clone, Copy)]
struct ExecOptions {
    depth: Depth,
    quiet: bool,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Leaf {
    #[serde(flatten, default)]
    custom: HashMap<String, Steps>,

    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Leaf {
    fn from_yaml<R: std::io::Read>(source: R) -> Result<Self> {
        serde_yaml::from_reader(source).context(FailedToReadFernFile {})
    }

    fn path(&self) -> PathBuf {
        self.path.clone().unwrap()
    }

    fn from_file(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Result::Err(Error::NoLeafFoundHere);
        }

        let file = File::open(path.clone()).unwrap();

        let mut config = Leaf::from_yaml(file)?;

        config.path = Some(path);

        Ok(config)
    }

    fn operations(&self) -> HashSet<String> {
        let mut operations = HashSet::new();
        for (op, steps) in &self.custom {
            if steps.any() {
                operations.insert(op.to_string());
            }
        }

        operations
    }

    fn run(self, op: &Operation) -> Result<()> {
        let steps = self
            .custom
            .get(&op.0)
            .cloned()
            .unwrap_or_else(Steps::default);

        let file_path = self.path.unwrap();
        let cwd = file_path.parent().unwrap();
        run_all_steps(steps, &cwd)
    }
}

fn find_fern_files() -> Vec<PathBuf> {
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

fn style(mut args: Arguments) -> PrintStyle {
    if args.contains(["-p", "--porcelain"]) {
        PrintStyle::Porcelain
    } else {
        PrintStyle::Pretty
    }
}

fn command() -> Options {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-v", "--version"]) {
        return Options::Version;
    }

    if let Ok(Some(cmd)) = args.subcommand() {
        match cmd.as_str() {
            "help" => return Options::Help,
            "leaves" => return Options::Leaves(style(args)),
            "seed" => {
                let language = args.subcommand().ok().flatten();
                return Options::Seed { language };
            }
            "list" => return Options::List(style(args)),
            other => return Options::Exec(Operation(other.to_owned()), opts(args)),
        }
    }
    Options::Help
}

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
        Options::List(style) => print_list_of_operations(style),
    };

    if let Result::Err(e) = res {
        println!("{}", e);
        process::exit(-1);
    }
}

fn print_list_of_operations(style: PrintStyle) -> Result<()> {
    let leaves = all_leaves()?;

    let mut operations = HashSet::new();

    for leaf in leaves {
        operations = operations
            .union(&leaf.operations())
            .map(|s| s.to_string())
            .collect();
    }

    let mut operations: Vec<String> = operations.into_iter().collect();
    operations.sort();

    match style {
        PrintStyle::Pretty => {
            println!("Available commands are");
            for op in operations {
                println!(" * {}", op)
            }
        }
        PrintStyle::Porcelain => {
            for op in operations {
                println!("{}", op)
            }
        }
    };

    Ok(())
}

fn all_leaves() -> Result<Vec<Leaf>> {
    let mut leaves = Vec::new();
    for leaf in find_fern_files() {
        leaves.push(Leaf::from_file(leaf)?);
    }

    Ok(leaves)
}

fn print_version() -> Result<()> {
    println!("fern version {}", env!("CARGO_PKG_VERSION"));
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
        fmt         for anything formatting related
        build       for anything related to building the app
        test        for running any kind of tests
        check       for things like type-checks or build-checks
        leaves      for showing all fern.yaml files. Has a -p | --porcelain option for better tooling
        seed        for seeding new fern.yaml files based on a config

    [OPTIONS]
        here        to only look in the current dir for a fern.yaml file, not recurisively searching the entire tree 
        -q | --quiet  to silence errors when no fern.yaml file is present

    Examples
        $: fern fmt  # will look for all fern.yaml files and run the 'fmt' target
        $: fern fmt here  # will look only use the one in the current directory

    Configuration
        fern will look in $HOME/.fern.config.yaml or in $FERN_CONFIG for a configuration
        file for seeding.
        A sample config file looks like this:

        seeds:
          node:
            test: npm test
            fmt:  npm run prettier
          rust:
            test:  cargo test
            fmt:   cargo fmt
            check: cargo check
            build: cargo build --release

    Any other input will print this help menu.
   "#
    );
    Ok(())
}

fn run_leaves(op: Operation, opts: ExecOptions) -> Result<()> {
    if opts.depth == Depth::Recursive {
        let leaves = all_leaves()?;
        if leaves.is_empty() {
            Result::Err(Error::NoLeafFoundAnywhere)
        } else {
            for leaf in leaves {
                leaf.run(&op)?;
            }
            Ok(())
        }
    } else {
        let leaf = Leaf::from_file(PathBuf::from("./fern.yaml"))?;
        leaf.run(&op)
    }
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
    let leaves = all_leaves()?;
    match style {
        PrintStyle::Porcelain => {
            for leaf in leaves {
                println!("{}", leaf.path().to_string_lossy())
            }
        }
        PrintStyle::Pretty => {
            println!("Considering leaves:");
            for leaf in leaves {
                println!(" *\t{}", leaf.path().to_string_lossy())
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
    use crate::Leaf;

    #[test]
    fn it_parses_a_correct_yaml_file() {
        let yaml = r#"
       fmt: Something
       build: Fancy
       "#;

        let folder = Leaf::from_yaml(yaml.as_bytes()).unwrap();

        let possible_operations = folder.operations();

        assert!(possible_operations.contains("fmt"));
        assert!(possible_operations.contains("build"));
    }

    #[test]
    fn it_reports_adequate_errors() {
        let yaml = r#"fmt: Something
        has no value:
       "#;

        let error = Leaf::from_yaml(yaml.as_bytes()).unwrap_err().to_string();

        assert_eq!("There was an error when reading the file: mapping values are not allowed in this context at line 2 column 21", error)
    }

    #[test]
    fn it_reports_errors_for_known_keys() {
        let yaml = "fmt: 12";

        let error = Leaf::from_yaml(yaml.as_bytes()).unwrap_err().to_string();

        assert_eq!("There was an error when reading the file: invalid type: integer `12`, expected either single string or sequence of strings at line 1 column 4", error)
    }
}

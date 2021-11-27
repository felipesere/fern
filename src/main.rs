use std::collections::HashSet;
use std::str::FromStr;

use leaf::Leaf;

mod leaf;
mod seed;
mod steps;

use anyhow::{Error, Result};

use clap::{ArgEnum, Parser};

/// Gives different parts of your mono-repo a unified interface to run certain tasks
///
///
/// Any other input will print this help menu.
///
/// Any key you have defined in your fern files can be run as a subcommand.
/// Common examples could be 'test', 'build', 'check', 'fmt', or 'lint'.
#[derive(Parser)]
#[clap(
    name = "fern",
    version = "0.0.3",
    author = "Felipe Sere <fern@felipesere.com>",
    setting = clap::AppSettings::DeriveDisplayOrder,
    after_help = include_str!("../after_help.txt"),
)]
struct Opts {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Parser)]
enum Cmd {
    /// for showing all fern.yaml files.
    Leaves {
        #[clap(flatten)]
        style: Style,
    },

    /// for seeding new fern.yaml files based on a config
    Seed { language: Option<String> },

    /// for showing which commands are available across your fern files
    List {
        #[clap(flatten)]
        style: Style,
    },

    /// Run any command from your fern file
    #[clap(external_subcommand)]
    Exec(Vec<String>),
}

impl FromStr for Operation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Operation(s.to_string()))
    }
}

/// Yay?
#[derive(Parser)]
struct Style {
    /// Don't print any special list characters. Useful for scripts.
    #[clap(long, short)]
    porcelain: bool,
}

#[derive(ArgEnum, Clone)]
enum PrintStyle {
    Pretty,
    Porcelain,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    match opts.cmd {
        Cmd::Leaves { style } => {
            let style = if style.porcelain {
                PrintStyle::Porcelain
            } else {
                PrintStyle::Pretty
            };
            print_leaves(style)
        }
        Cmd::Seed { language } => seed::folder(language),
        Cmd::List { style } => {
            let style = if style.porcelain {
                PrintStyle::Porcelain
            } else {
                PrintStyle::Pretty
            };
            print_list_of_operations(style)
        }
        Cmd::Exec(args) => {
            let operation = args[0].to_string();
            let options = ExecOptions::parse_from(&args[1..]);
            run_leaves(Operation(operation), options).or_else(options.quietly())
        }
    }
}

pub(crate) struct Operation(String);

#[derive(Parser, Clone, Copy, Debug)]
#[clap(setting = clap::AppSettings::NoBinaryName)] // allows me to use `parse_from` to re-parse the arguments
struct ExecOptions {
    /// Only operate on the fern file found directly in this directory
    #[clap(short, long)]
    here: bool,

    /// Don't print any errors if there is no fern file here. Practical for scripts.
    #[clap(short, long)]
    quiet: bool,
}

impl ExecOptions {
    fn quietly(self) -> impl FnOnce(Error) -> Result<()> {
        move |e| {
            if self.quiet {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn print_list_of_operations(style: PrintStyle) -> Result<()> {
    let leaves = Leaf::all_leaves()?;

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

fn run_leaves(operation: Operation, opts: ExecOptions) -> Result<()> {
    if !opts.here {
        for leaf in Leaf::all_leaves()? {
            leaf.run(&operation)?;
        }
        Ok(())
    } else {
        Leaf::here()?.run(&operation)
    }
}

fn print_leaves(style: PrintStyle) -> Result<()> {
    let leaves = Leaf::all_leaves()?;
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

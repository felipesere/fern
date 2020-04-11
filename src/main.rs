use std::collections::HashSet;

use leaf::Leaf;

mod arguments;
mod leaf;
mod seed;
mod steps;

use anyhow::{Error, Result};

fn main() -> Result<()> {
    use Options::*;
    match arguments::parse() {
        Version => print_version(),
        Help => print_help(),
        Leaves(style) => print_leaves(style),
        Seed(language) => seed::folder(language),
        List(style) => print_list_of_operations(style),
        Exec(command, opts) => run_leaves(command, opts).or_else(opts.quietly()),
    }
}

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
    Seed(Option<String>),
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

impl ExecOptions {
    fn quietly(self) -> impl FnOnce(Error) -> Result<()> {
        return move |e| {
            if self.quiet {
                Ok(())
            } else {
                Err(e)
            }
        };
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
    if opts.depth == Depth::Recursive {
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

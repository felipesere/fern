use crate::{Depth, ExecOptions, Operation, Options, PrintStyle};
use pico_args::Arguments;

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

pub(crate) fn parse() -> Options {
    let mut args = pico_args::Arguments::from_env();

    if args.contains(["-v", "--version"]) {
        return Options::Version;
    }

    if let Ok(Some(cmd)) = args.subcommand() {
        match cmd.as_str() {
            "help" => Options::Help,
            "leaves" => Options::Leaves(style(args)),
            "seed" => {
                let language = args.subcommand().ok().flatten();
                Options::Seed(language)
            }
            "list" => Options::List(style(args)),
            other => Options::Exec(Operation(other.to_owned()), opts(args)),
        }
    } else {
        Options::Help
    }
}

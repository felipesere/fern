#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};

// re-export for all modules to share
pub use predicates::{prelude::PredicateBooleanExt, str::contains as c};
pub(crate) struct Dir {
    v: &'static str,
}

impl Dir {
    pub fn run(self, cli: &'static str) -> Assert {
        let mut fern = Command::cargo_bin("fern").unwrap();
        fern.current_dir(self.v);

        let args = cli.split(" ").into_iter().skip(1).collect::<Vec<_>>();
        fern.args(args);

        fern.assert()
    }
}

pub(crate) fn run(cli: &'static str) -> Assert {
    cd("./").run(cli)
}

pub(crate) fn cd(dir: &'static str) -> Dir {
    Dir { v: dir }
}

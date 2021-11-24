#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};

// re-export for all modules to share
pub use predicates::{prelude::PredicateBooleanExt, str::contains as c};
use std::path::PathBuf;
pub(crate) struct Dir {
    v: PathBuf,
    env: String,
}

impl Dir {
    pub fn run(self, cli: &'static str) -> Assert {
        let mut fern = Command::cargo_bin("fern").unwrap();

        if self.env != "" {
            if let [key, value] = &self.env.split('=').collect::<Vec<&str>>()[..] {
                fern.env(key, value);
            }
        }

        fern.current_dir(self.v);

        let args = cli.split(' ').skip(1).collect::<Vec<_>>();
        fern.args(args);

        fern.assert()
    }

    pub fn env<S: Into<String>>(mut self, env: S) -> Dir {
        self.env = env.into();

        self
    }
}

pub(crate) fn run(cli: &'static str) -> Assert {
    cd("./").run(cli)
}

pub(crate) fn cd<P: Into<PathBuf>>(dir: P) -> Dir {
    let dir: PathBuf = dir.into();
    if !dir.exists() {
        panic!("could not 'cd' into non-existing file: {:?}", dir);
    }
    Dir {
        v: dir,
        env: "".into(),
    }
}

# Fern

[![build](https://github.com/felipesere/fern/actions/workflows/ci.yaml/badge.svg)](https://github.com/felipesere/fern/actions/workflows/ci.yaml)
[![Coverage](https://codecov.io/gh/felipesere/fern/branch/master/graph/badge.svg)](https://codecov.io/gh/felipesere/fern)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/fern-run)](https://crates.io/crates/fern-run) 

`fern` is something like a command runner. You can drop `fern.yaml` files into differnet parts of your (mono-) repo and `fern` will detect them and run the command that you want from them.
As such, it gives different parts of your mono-repo a unified interface to run certain tasks.

Have a look at this blog by Jeff Ramnani [Project Build Tool](https://8thlight.com/blog/jeff-ramnani/2017/08/07/project-build-protocol.html) for the idea.

## Context - Files where they make sense

Say you have a larger project, composed of multiple smaller parts.
Each of those parts could be in a different language.
Maybe your backend is written in Rust, while the mail service is a Python app, and the frontend is written in ELM.

Now, you could write a `Makefile` that orchestrates across all the apps,
launching `cargo`,`pip`, and `elm` in just the right folders.
The thing I found annoying, is that one `Makefile` rules them all.
That `Makefile` mixes and matches concerns across many languages and build tools _and_ it needs to get clever with folders doing things like `cd ui && npm install`.

`fern` is a little different - and orders of magnitude _less_ powerful than make - but also simpler.
`fern` can find `fern.yaml` files spread throughout your code base.
In our example, we'd have one for the Rust backend, one for the ELM frontend, and a separate one for Python.
If you want to know which files `fern` would consider, run `fern leaves`. 
Because a `fern` has many leaves :smile:.

## The leaves make the fern

I have lovingly touted `fern.yaml` files as `leaves`, which is also the name of the command to list them.
In such a fern `fern.yaml` file you would for example define these 4 targets:

* `fmt` for anything formatting related
* `build` for anything related to building the app
* `test` for running any kind of tests
* `check` for things like type-checks or build-checks

You are allowed to write single lines like so:
```yaml
fmt: cargo fmt
test: cargo test
```

or use lists for multiple steps:
```yaml
fmt:
 - npm run fmt
 - prettier --write src/css/*.css
test: npm test
check:
 - tsc
 - prettier --check {src,test}/**/*.tsx
 - prettier --check src/css/*.css
```

That is it. 
There is no way to describe interdependencies or any intricate ordering between files.



If those 4 sample targets don't suite your needs, you can add more as you see fit.

## Running it

The commands match exactly what you'd write in the fern file.

For the example above:

* `fern fmt` for anything formatting related
* `fern build` for anything related to building the app
* `fern test` for running any kind of tests
* `fern check` for things like type-checks or build-checks

There are a few helper commands that make exploring `fern` a little easier.

`leaves` shows you which `fern` files it would find.
You can give the argument `-p` or `--porcelain` to get all files in a single line, which is nice for opening them all in vim like so:

```
vim -p $(fern leaves -p)
```

`list` is super practical to find out what commands you have defined across all `fern.yaml` files.
It list all commands across all fern files and gives you a unique list. It does not matter if a command is only defined in a far far-away `fern.yaml` file:

```bash
fern list
Available commands are
 * apple
 * banana
 * build
 * check
 * fmt
 * run
 * test
```



Here is a demo of `fern` running in a different repo of mine:
[![asciicast](https://asciinema.org/a/QbKh6hrb8I8bnmvMcSDq3PHkP.png)](https://asciinema.org/a/QbKh6hrb8I8bnmvMcSDq3PHkP)

## Seeding ferns and configuration
Using `fern` should require as little configuration as possible, especially since its feature-set is so small.
There is one feature that needs a global configuration file though: `fern seed $name`.
`seed` will create a `fern.yaml` file for you, based on the `$name` and what is globally configured.
The configuration file is expected in `$HOME/.fern.config.yaml` (`$HOME` will vary per OS) but you can change it using the
`FERN_CONFIG` environment variable.

Here is a sample config file:
```yaml
seeds:
  rust:
     fmt: cargo fmt
     test: cargo test
     build: argo build --release
  elixir:
     fmt: mix format .
     test: mix test
```

Under the key `seeds` you can name different chunks that look like a `fern.yaml` file.
Running `fern seed rust` will then copy that chunk into a local `fern.yaml`.
This is practical if you have multiple projects that need similar configs.

## Anti-feature

These are things that `fern` is not meant to do and will likely never learn, mostly for sake of simplicity:

* Dependency ordering and tracking: `fern` us just meant to run commands. Coming up with some kind of dependency tree or explicit ordering would make it more complex. 
* Caching build outputs: this is extremely complex and there are tools that do this way better (e.g. `bazel` and friends)



## Installing and contributing

At the moment, the fastest way to install it is using `cargo` from the Rust distribution:

```bash
cargo install fern-run
```

I had to rename it to `fern-run` on crates.io as the name fern was already taken. The command it installs is still called `fern` though.

Contributions are super welcome:
* There are no tests, so feel free to write any
* Are there any lightweight features you think would benefit`fern`? Open an issue or PR

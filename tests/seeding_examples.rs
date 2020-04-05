use assert_fs::assert::PathAssert;
use assert_fs::fixture::ChildPath;
use utils::*;

#[path = "./utils.rs"]
mod utils;

fn cleanup<F>(file: &'static str, test: F)
where
    F: FnOnce() -> (),
{
    let _ = std::fs::remove_file(file);

    test();

    let _ = std::fs::remove_file(file);
}

fn fern_config(relative: &'static str) -> String {
    let mut config_file = std::env::current_dir().unwrap();
    config_file.push(relative);
    format!("FERN_CONFIG={}", config_file.to_string_lossy())
}
#[test]
fn it_can_seed_a_project_based_on_config() {
    cleanup("./example/empty/fern.yaml", || {
        let assert = cd("./example/empty")
            .env(fern_config("example/fern.config.yaml"))
            .run("fern seed rust");

        assert
            .success()
            .stdout(c("Created new fern.yaml file for rust"));

        ChildPath::new("./example/empty/fern.yaml").assert(
            r#"---
fmt: "echo \"cargo fmt\""
test: "echo \"cargo test\""
build: "echo \"cargo build --release\"""#,
        );
    });
}

#[test]
fn it_doesnt_seed_an_unknown_language() {
    let assert = cd("./example/empty")
        .env(fern_config("example/fern.config.yaml"))
        .run("fern seed node");

    assert.failure().stderr(c("Did not find node in config"));
}

#[test]
fn it_tells_the_user_if_the_config_can_not_parsed() {
    let assert = cd("./example/empty")
        .env(fern_config("example/not-really.yaml"))
        .run("fern seed node");

    assert
        .failure()
        .stderr(c("Unable to read configuration").and(c("fern/example/not-really.yaml")));
}

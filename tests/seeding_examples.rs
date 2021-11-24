use assert_fs::assert::PathAssert;
use assert_fs::fixture::ChildPath;
use assert_fs::TempDir;
use utils::*;

#[path = "./utils.rs"]
mod utils;

fn fern_config(relative: &'static str) -> String {
    let mut config_file = std::env::current_dir().unwrap();
    config_file.push(relative);
    format!("FERN_CONFIG={}", config_file.to_string_lossy())
}
#[test]
fn it_can_seed_a_project_based_on_config() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let assert = cd(&dir)
        .env(fern_config("example/fern.config.yaml"))
        .run("fern seed rust");

    assert
        .success()
        .stdout(c("Created new fern.yaml file for rust"));

    ChildPath::new(dir.join("fern.yaml")).assert(
        r#"---
fmt: "echo \"cargo fmt\""
test: "echo \"cargo test\""
build: "echo \"cargo build --release\""
"#,
    );
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

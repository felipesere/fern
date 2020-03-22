use utils::*;

#[path = "./utils.rs"]
mod utils;

#[test]
fn it_can_seed_a_project_based_on_config() {
    let assert = cd("./example/empty")
        .env("FERN_CONFIG=../fern.config.yaml")
        .run("fern seed rust");
    assert
        .success()
        .stdout(c("Created new fern.yaml file for rust"));
}

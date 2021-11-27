use utils::*;

#[path = "./utils.rs"]
mod utils;

#[test]
fn it_can_run_custom_commands() {
    let assert = cd("./example/custom").run("fern run --here");

    assert.success().stdout(c("this is non-standard!"));
}

#[test]
fn it_can_list_all_available_commands() {
    let assert = cd("./example/custom").run("fern list");

    assert.success().stdout(
        c("Available commands are")
            .and(c("* run"))
            .and(c("* apple"))
            .and(c("* banana")),
    );
}

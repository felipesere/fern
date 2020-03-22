use utils::*;

#[path = "./utils.rs"]
mod utils;

#[test]
fn it_prints_the_version() {
    let assert = run("fern -v");

    assert.success().stdout(c("fern version"));
}

#[test]
fn it_prints_some_help() {
    let assert = run("fern help");

    assert.success().stdout(c(
        "Gives different parts of your mono-repo a unified interface to run certain tasks.",
    ));
}
#[test]
fn it_runs_fmt_for_the_entire_directory() {
    let assert = cd("./example").run("fern fmt");

    assert.success().stdout(
        c("Running fmt from within foo/batz")
            .and(c("Running fmt from within batz"))
            .and(c("Running fmt from within bar")),
    );
}
#[test]
fn it_runs_build_for_the_current_folder() {
    let assert = cd("./example").run("fern build here");

    assert.success().stdout(c("Fooooo"));
}

#[test]
fn runs_multiple_tests_from_the_fern_file() {
    let assert = cd("./example").run("fern fmt here");

    assert.success().stdout(c("The first").and(c("The second")));
}

#[test]
fn it_list_all_available_leaves() {
    let assert = cd("./example").run("fern leaves");

    assert.success().stdout(
        c("Considering leaves:")
            .and(c(" *\t./foo/batz/fern.yaml"))
            .and(c(" *\t./bar/batz/fern.yaml"))
            .and(c("./bar/fern.yaml")),
    );
}

#[test]
fn it_prints_the_leaves_woithout_extra_formatting() {
    let assert = cd("./example").run("fern leaves -p");

    assert.success().stdout(
        c("./foo/batz/fern.yaml")
            .and(c("./bar/batz/fern.yaml"))
            .and(c("./bar/fern.yaml")),
    );
}

#[test]
fn it_warns_if_there_are_no_fern_files_anywhere() {
    let assert = cd("./example/empty").run("fern fmt");

    assert
        .failure()
        .stdout(c("Did not find any fern.yaml file"));
}

#[test]
fn it_warns_if_there_is_no_fern_file_here() {
    let assert = cd("./example/empty").run("fern fmt here");

    assert
        .failure()
        .stdout(c("Did not find a fern.yaml file in here"));
}

#[test]
fn it_allows_the_user_to_suppress_the_missing_file_warning() {
    let assert = cd("./example/empty").run("fern fmt here -q");

    assert.success().stdout(predicates::str::is_empty());
}

#[test]
fn it_reports_when_commands_fail() {
    let assert = cd("./example").run("fern check here");

    assert
        .failure()
        .stdout(c(
            "Failed to execute command 'does not exist': exit code 127",
        ))
        .stderr(c("sh:").and(c("does:")).and(c("not found")));
}

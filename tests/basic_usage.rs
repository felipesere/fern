use assert_cmd::Command;
use predicates::{prelude::PredicateBooleanExt, str::contains as c};

#[test]
fn it_prints_the_version() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd.arg("-v").assert();

    assert.success().stdout(c("fern version"));
}
#[test]
fn it_runs_fmt_for_the_entire_directory() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd.current_dir("./example").arg("fmt").assert();

    assert.success().stdout(
        c("Running fmt from within foo/batz")
            .and(c("Running fmt from within batz"))
            .and(c("Running fmt from within bar")),
    );
}
#[test]
fn it_runs_fmt_for_the_current_folder() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd.arg("fmt").arg("here").assert();

    assert.success().stdout("running fmt\n");
}

#[test]
fn it_list_all_available_leaves() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd.current_dir("./example").arg("leaves").assert();

    assert.success().stdout(
        c("Considering leaves:")
            .and(c(" *\t./foo/batz/fern.yaml"))
            .and(c(" *\t./bar/batz/fern.yaml"))
            .and(c("./bar/fern.yaml")),
    );
}

#[test]
fn it_prints_the_leaves_woithout_extra_formatting() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd
        .current_dir("./example")
        .arg("leaves")
        .arg("-p")
        .assert();

    assert.success().stdout(
        c("./foo/batz/fern.yaml")
            .and(c("./bar/batz/fern.yaml"))
            .and(c("./bar/fern.yaml")),
    );
}

#[test]
fn it_warns_if_there_are_no_fern_files_anywhere() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd.current_dir("./example/empty").arg("fmt").assert();

    assert
        .success()
        .stdout(c("Did not find any fern.yaml file"));
}

#[test]
fn it_warns_if_there_is_no_fern_file_here() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd
        .current_dir("./example/empty")
        .arg("fmt")
        .arg("here")
        .assert();

    assert
        .success()
        .stdout(c("Did not find a fern.yaml file in here"));
}

#[test]
fn it_allows_the_user_to_suppress_the_missing_file_warning() {
    let mut cmd = Command::cargo_bin("fern").unwrap();

    let assert = cmd
        .current_dir("./example/empty")
        .arg("fmt")
        .arg("here")
        .arg("-q")
        .assert();

    assert.success().stdout(predicates::str::is_empty());
}

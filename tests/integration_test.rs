extern crate challenge;

use assert_cmd::Command;
use pretty_assertions::assert_eq;

use std::fs;

#[test]
fn test_successful_run() {
    // here we're going to actually create our CSV file and save it to a tmp file
    let input = concat!(
        "type,client,tx,    amount\n",
        "deposit,1, 1, 1\n",
        "deposit,2,2,2\n",
        "deposit,1,3,   2\n",
        "withdrawal,1,4     ,1   \n",
        "withdrawal,2,5,3\n",
    );
    let expected_output = concat!(
        "client,available,held,total,locked\n",
        "1,2,0,2,false\n",
        "2,2,0,2,false\n"
    );
    let tmp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    fs::write(tmp_file.path(), input).expect("Failed to write to temp file");

    let mut cmd = Command::cargo_bin("challenge").unwrap();
    let output = cmd
        .arg(tmp_file.path())
        .output()
        .expect("Expected no errors");

    assert_eq!(output.status.code(), Some(0));

    let output_str = String::from_utf8(output.stdout).expect("Not UTF-8");
    assert_eq!(output_str, expected_output);
}

#[test]
fn test_invalid_args() {
    let mut cmd = Command::cargo_bin("challenge").unwrap();
    let output = cmd.output().expect("Expected no errors");

    assert_eq!(output.status.code(), Some(1));

    let output_str = String::from_utf8(output.stderr).expect("Not UTF-8");
    assert!(
        output_str.contains("Usage:"),
        "Expected usage message, got: {}",
        output_str
    );
}

#[test]
fn test_file_not_found() {
    let mut cmd = Command::cargo_bin("challenge").unwrap();
    let output = cmd
        .arg("/tmp/does-not-exist")
        .output()
        .expect("Expected no errors");

    assert_eq!(output.status.code(), Some(1));

    let output_str = String::from_utf8(output.stderr).expect("Not UTF-8");
    assert!(
        output_str.contains("No such file"),
        "Expected file not found message, got: {}",
        output_str
    );
}

#[test]
fn test_malformed_csv() {
    // here we're going to actually create our CSV file and save it to a tmp file
    let input = concat!(
        "type,client,tx,    amount\n",
        "deposit,1, 1, 1,UNEXPECTED\n",
        "deposit,2,2,2\n",
    );

    let tmp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    fs::write(tmp_file.path(), input).expect("Failed to write to temp file");

    let mut cmd = Command::cargo_bin("challenge").unwrap();
    let output = cmd
        .arg(tmp_file.path())
        .output()
        .expect("Expected no errors");

    assert_eq!(output.status.code(), Some(1));

    let output_str = String::from_utf8(output.stderr).expect("Not UTF-8");
    assert_eq!(output_str, "Error: \"CSV error: record 1 (line: 2, byte: 26): found record with 5 fields, but the previous record has 4 fields\"\n");
}

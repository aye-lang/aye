//! A closed stdout (`aye spec | head`) must end the command quietly: no
//! "failed printing to stdout" panic text on stderr.

use std::io::Read;
use std::process::{Command, Stdio};

fn closed_pipe_stderr(args: &[&str]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aye"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aye");
    // Read a few bytes, then close the read end while output is still pending.
    let mut out = child.stdout.take().unwrap();
    let mut head = [0u8; 16];
    out.read_exact(&mut head).expect("read head");
    drop(out);
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    child.wait().unwrap();
    err
}

#[test]
fn spec_with_closed_stdout_is_quiet() {
    let err = closed_pipe_stderr(&["spec"]);
    assert_eq!(err, "", "stderr must be empty");
}

#[test]
fn doc_signatures_with_closed_stdout_is_quiet() {
    let err = closed_pipe_stderr(&["doc", "--signatures", "std"]);
    assert_eq!(err, "", "stderr must be empty");
}

//! Opt-in page-load timing and immutable response archives.
use super::{anyknown, root, scratch, server, text};
use std::process::Command;

#[test]
fn timing_preserves_output() {
    let args = ["dump", "--md", "--js", "browser/tests/agent/noscript.html"];
    let plain = Command::new(anyknown()).current_dir(root()).args(args).output().unwrap();
    let timed = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_TIMING", "1").args(args).output().unwrap();
    assert!(plain.status.success() && plain.stderr.is_empty(), "{}", text(&plain.stderr));
    assert!(timed.status.success(), "{}", text(&timed.stderr));
    assert_eq!(plain.stdout, timed.stdout);
    let trace = text(&timed.stderr);
    for phase in ["fetch", "decode", "html", "scripts", "css_parse", "cascade", "layout", "markdown"] {
        assert!(trace.contains(&format!("ANYKNOWN_TIMING\t{phase}\t")), "missing {phase}: {trace}");
    }
    assert!(!trace.contains("leak"), "{trace}");
}

#[test]
fn response_archive_replays_without_server() {
    let Some(srv) = server() else { return };
    let dir = scratch().join("perf-archive");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // External CSS (including a redirect), JS, and an image. The server
    // serves /log/* with appropriate content types and counts requests.
    let page = "data:text/html,<link rel=stylesheet href='BASE/log/style.css'><script src='BASE/log/script.js'></script><p id=x>Archived</p><img src='BASE/log/image.png'><script>document.getElementById('x').textContent += fromScript</script>".replace("BASE", &srv.base);
    let args = ["dump", "--md", "--js", &page];
    let recorded = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_RECORD", &dir).args(args).output().unwrap();
    assert!(recorded.status.success() && recorded.stderr.is_empty(), "{}", text(&recorded.stderr));
    assert!(std::fs::read_dir(&dir).unwrap().count() >= 4);
    let redirect = format!("{}/redirect", srv.base);
    let recorded_redirect = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_RECORD", &dir).args(["dump", "--md", "--js", &redirect]).output().unwrap();
    assert!(recorded_redirect.status.success() && recorded_redirect.stderr.is_empty(), "{}", text(&recorded_redirect.stderr));
    // Replay must not reach the network, even with HTTP prefetch enabled.
    drop(srv);
    let replayed = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_REPLAY", &dir).args(args).output().unwrap();
    assert!(replayed.status.success() && replayed.stderr.is_empty(), "{}", text(&replayed.stderr));
    assert_eq!(recorded.stdout, replayed.stdout);
    let replayed_redirect = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_REPLAY", &dir).args(["dump", "--md", "--js", &redirect]).output().unwrap();
    assert!(replayed_redirect.status.success() && replayed_redirect.stderr.is_empty(), "{}", text(&replayed_redirect.stderr));
    assert_eq!(recorded_redirect.stdout, replayed_redirect.stdout);
    let missing = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_REPLAY", &dir).args(["dump", "--md", "--js", "http://127.0.0.1:1/missing"]).output().unwrap();
    assert!(!missing.status.success());
    assert!(text(&missing.stderr).contains("archive miss:"), "{}", text(&missing.stderr));
    std::fs::remove_dir_all(&dir).unwrap();
}

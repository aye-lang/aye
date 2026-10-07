//! Opt-in page-load timing and immutable response archives.
use super::{anyknown, root, scratch, server, text};
use std::process::Command;

#[test]
fn live_frame_index_follows_mutations() {
    let page = "data:text/html,<p id=out></p><script>let r=[window.length];let a=document.createElement('iframe');a.setAttribute('name','a');document.body.appendChild(a);r.push(window.length);let b=document.createElement('iframe');b.setAttribute('name','b');document.body.appendChild(b);r.push(window.length);document.body.insertBefore(b,a);r.push(window.frames[0]===b.contentWindow,window.frames[1]===a.contentWindow);b.remove();r.push(window.length);a.remove();r.push(window.length);document.getElementById('out').textContent=r.join(',');</script>";
    let out = Command::new(anyknown()).current_dir(root()).args(["dump", "--text", "--js", page]).output().unwrap();
    assert!(out.status.success() && out.stderr.is_empty(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "0,1,2,true,true,1,0\n");
}

#[test]
fn cached_sheets_follow_media_changes() {
    let page = "data:text/html,<style id=css>p{color:red}</style><p id=out></p><script>let p=document.getElementById('out');let css=document.getElementById('css');let r=[getComputedStyle(p).color];document.head.appendChild(document.createElement('style'));r.push(getComputedStyle(p).color);css.setAttribute('media','(max-width:1px)');r.push(getComputedStyle(p).color);p.textContent=r.join('|');</script>";
    let out = Command::new(anyknown()).current_dir(root()).args(["dump", "--text", "--js", page]).output().unwrap();
    assert!(out.status.success() && out.stderr.is_empty(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "rgb(255, 0, 0)|rgb(255, 0, 0)|rgb(0, 0, 0)\n");
}

#[test]
fn script_layout_is_reused_until_a_change() {
    let page = "data:text/html,<div id=d>x</div><p id=out></p><script>let d=document.getElementById('d');let r=[d.offsetWidth,d.offsetHeight,d.getBoundingClientRect().width,d.clientWidth];d.style.width='10px';r.push(d.offsetWidth,d.getBoundingClientRect().width);document.getElementById('out').textContent=r.join(',');</script>";
    let out = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_TIMING", "1").args(["dump", "--text", "--js", "--no-scroll", page]).output().unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "x\n\n784,20,784,784,10,10\n");
    let trace = text(&out.stderr);
    assert_eq!(trace.matches("ANYKNOWN_TIMING\tscript_layout\t").count(), 2, "{trace}");
}

/// A script that writes and reads in turns (as frameworks do) forces a
/// layout per read, but the document is styled whole only once: each
/// later layout styles again what the change reaches. Changes to a tree
/// out of the document force none.
#[test]
fn interleaved_writes_and_reads_restyle_partially() {
    let page = "data:text/html,<style>.on{height:5px}</style><div id=l></div><p id=out></p><script>let l=document.getElementById('l');for(let i=0;i<20;i++)l.append(document.createElement('div'));let loose=document.createElement('div'),sum=0;for(const c of l.children){c.className='on';loose.append(document.createElement('i'));sum+=l.offsetHeight;}document.getElementById('out').textContent=sum;</script>";
    let out = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_TIMING", "1").args(["dump", "--text", "--js", "--no-scroll", page]).output().unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "1050\n");
    let trace = text(&out.stderr);
    assert_eq!(trace.matches("ANYKNOWN_TIMING\tscript_layout\t").count(), 20, "{trace}");
    assert_eq!(trace.matches("ANYKNOWN_TIMING\trestyle\t").count(), 19, "{trace}");
    assert!(trace.matches("ANYKNOWN_TIMING\tcascade\t").count() <= 2, "{trace}");
}

#[test]
fn timing_preserves_output() {
    let args = ["dump", "--md", "--js", "--no-scroll", "browser/tests/agent/noscript.html"];
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
    assert_eq!(trace.matches("ANYKNOWN_TIMING\tcascade\t").count(), 1, "{trace}");
}

#[test]
fn timing_preserves_paint() {
    let plain = scratch().join("perf-plain.png");
    let timed = scratch().join("perf-timed.png");
    let args = ["screenshot", "browser/tests/paint/text.html", "--font", "fixed", "-o"];
    let first = Command::new(anyknown()).current_dir(root()).args(args).arg(&plain).output().unwrap();
    let second = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_TIMING", "1").args(args).arg(&timed).output().unwrap();
    assert!(first.status.success() && first.stderr.is_empty(), "{}", text(&first.stderr));
    assert!(second.status.success(), "{}", text(&second.stderr));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(std::fs::read(plain).unwrap(), std::fs::read(timed).unwrap());
    let trace = text(&second.stderr);
    for phase in ["paint_build", "paint"] {
        assert!(trace.contains(&format!("ANYKNOWN_TIMING\t{phase}\t")), "{trace}");
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
    // The stylesheet, script and image; not the data: page, which holds
    // its own response.
    assert!(std::fs::read_dir(&dir).unwrap().count() >= 3);
    let failure_url = "http://127.0.0.1:1/unavailable";
    let recorded_failure = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_RECORD", &dir).args(["dump", "--md", "--js", failure_url]).output().unwrap();
    assert!(!recorded_failure.status.success());
    let replayed_failure = Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_REPLAY", &dir).args(["dump", "--md", "--js", failure_url]).output().unwrap();
    assert_eq!(recorded_failure.stdout, replayed_failure.stdout);
    assert_eq!(recorded_failure.stderr, replayed_failure.stderr);
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

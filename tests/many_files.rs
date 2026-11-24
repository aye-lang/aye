//! A panic names its file in a program of more than 2048 files (the
//! file's index once shared an `int` with the line and overflowed it).

use std::process::Command;

#[test]
fn a_panic_in_file_2101_names_the_file() {
    let dir = std::env::temp_dir().join(format!("aye-many-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 1..=2100 {
        let f = format!("pub fn f{i:04}() u32 {{ return {i} }}\n");
        std::fs::write(dir.join(format!("f{i:04}.aye")), f).unwrap();
    }
    let main = "pub fn main() {
    var a = [2]u32{1, 2}
    var i: usize = 0

    while i < 3 {
        @println(a[i])
        i += 1
    }
}
";
    std::fs::write(dir.join("zz.aye"), main).unwrap();
    for mode in ["debug", "safe"] {
        let exe = dir.join(format!("out-{mode}"));
        let built = Command::new(env!("CARGO_BIN_EXE_aye"))
            .current_dir(&dir)
            .args(["build", ".", "--mode", mode, "-o"])
            .arg(&exe)
            .output()
            .unwrap();
        assert!(
            built.status.success() && built.stderr.is_empty(),
            "aye build --mode {mode}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        let out = Command::new(&exe).output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.ends_with("zz.aye:6:19: panic: index out of bounds: index 2, len 2\n"),
            "--mode {mode}: {err}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

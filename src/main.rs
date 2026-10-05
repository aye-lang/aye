use ore::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "usage:
  ore build <file.ore> [-o out] [--mode debug|safe|fast]
  ore run <file.ore> [--mode debug|safe|fast]
  ore emit-c <file.ore> [--mode debug|safe|fast]";

struct Args {
    cmd: String,
    file: String,
    out: Option<String>,
    mode: Mode,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("missing command")?;
    let (mut file, mut out, mut mode) = (None, None, Mode::Debug);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" if cmd == "build" => out = Some(it.next().ok_or("`-o` needs a path")?),
            "--mode" => {
                let m = it.next().ok_or("`--mode` needs a value")?;
                mode = Mode::parse(&m).ok_or(format!("unknown mode `{m}`"))?;
            }
            _ if file.is_none() && !a.starts_with('-') => file = Some(a),
            _ => return Err(format!("unexpected argument `{a}`")),
        }
    }
    let file = file.ok_or("missing input file")?;
    Ok(Args { cmd, file, out, mode })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ore: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if !matches!(args.cmd.as_str(), "build" | "run" | "emit-c") {
        eprintln!("ore: unknown command `{}`\n{USAGE}", args.cmd);
        return ExitCode::from(2);
    }
    let src = match std::fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ore: cannot read {}: {e}", args.file);
            return ExitCode::FAILURE;
        }
    };
    let c = match driver::compile_to_c(&args.file, &src, args.mode) {
        Ok(c) => c,
        Err(d) => {
            eprintln!("{d}");
            return ExitCode::FAILURE;
        }
    };
    if args.cmd == "emit-c" {
        print!("{c}");
        return ExitCode::SUCCESS;
    }
    let exe = match (&args.cmd[..], &args.out) {
        ("build", Some(o)) => PathBuf::from(o),
        ("build", None) => PathBuf::from(Path::new(&args.file).file_stem().unwrap()),
        _ => std::env::temp_dir().join(format!("ore-run-{}", std::process::id())),
    };
    if let Err(e) = driver::cc(&c, &exe, args.mode) {
        eprintln!("ore: {e}");
        return ExitCode::FAILURE;
    }
    if args.cmd == "build" {
        return ExitCode::SUCCESS;
    }
    let status = Command::new(&exe).status();
    let _ = std::fs::remove_file(&exe);
    match status {
        Ok(s) => match s.code() {
            Some(code) => ExitCode::from(code as u8),
            None => {
                use std::os::unix::process::ExitStatusExt;
                ExitCode::from(128u8.wrapping_add(s.signal().unwrap_or(0) as u8))
            }
        },
        Err(e) => {
            eprintln!("ore: cannot run {}: {e}", exe.display());
            ExitCode::FAILURE
        }
    }
}

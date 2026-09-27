//! Executable stand-ins for the tests, written the way each OS can run
//! them. Shared by the fixture suites and, via `#[path]`, the unit tests.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// An agent stand-in on disk: runs `tests/fixtures/<fixture>/fixture.mjs` with
/// scenario flags, ignoring the real launch args appended after them. `name`
/// only keeps concurrent scenarios in separate temp dirs.
pub fn wrapper(fixture: &str, exe: &str, name: &str, flags: &str) -> PathBuf {
    let js = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture)
        .join("fixture.mjs");
    let dir =
        std::env::temp_dir().join(format!("anyagent-{fixture}-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    shim(&dir, exe, &js, flags)
}

/// A `.cmd` batch shim; `%*` forwards the args cmd.exe passes through.
#[cfg(windows)]
pub fn shim(dir: &Path, exe: &str, js: &Path, flags: &str) -> PathBuf {
    let path = dir.join(format!("{exe}.cmd"));
    let body = format!("@echo off\r\nnode \"{}\" {flags} %*\r\n", js.display());
    std::fs::write(&path, body).unwrap();
    path
}

/// A `#!/bin/sh` script carrying the execute bit.
#[cfg(unix)]
pub fn shim(dir: &Path, exe: &str, js: &Path, flags: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(exe);
    let body = format!("#!/bin/sh\nexec node '{}' {flags} \"$@\"\n", js.display());
    std::fs::write(&path, body).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// An installed executable that does nothing but exit 0, named so discovery's
/// suffix list finds it on either platform.
pub fn stub(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    write_stub(dir, name)
}

#[cfg(windows)]
fn write_stub(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(format!("{name}.cmd"));
    std::fs::write(&path, "@echo off\r\nexit /b 0\r\n").unwrap();
    path
}

#[cfg(unix)]
fn write_stub(dir: &Path, name: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The launch args each fixture process logged to the `FIXTURE_ARGV_LOG`
/// file the test set through `SessionOptions::env`, one list per process.
pub fn logged_args(log: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The sent frames in a `record_wire` log that match `pred`, polled until
/// `count` are there: the recorder writes in the background.
pub async fn sent_frames(
    log: &Path,
    count: usize,
    pred: impl Fn(&serde_json::Value) -> bool,
) -> Vec<serde_json::Value> {
    for _ in 0..80 {
        let frames: Vec<serde_json::Value> = std::fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["dir"] == "out")
            .map(|line| line["frame"].clone())
            .filter(|frame| pred(frame))
            .collect();
        if frames.len() >= count {
            return frames;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("fewer than {count} matching frames in {}", log.display());
}

/// The variable `std::env::home_dir` reads, for tests that redirect home.
#[cfg(unix)]
pub const HOME_VAR: &str = "HOME";
#[cfg(windows)]
pub const HOME_VAR: &str = "USERPROFILE";

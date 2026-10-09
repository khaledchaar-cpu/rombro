//! Commands to the running game's RetroArch over its stdin (`stdin_cmd_enable`), e.g.
//! switching the shader live. Only games started from Romburak are reachable.

use crate::commands::{CmdResult, err};
use std::io::Write;
use std::process::ChildStdin;
use std::sync::Mutex;

static RUNNING: Mutex<Option<(String, ChildStdin)>> = Mutex::new(None);

pub fn started(path: &str, stdin: Option<ChildStdin>) {
    if let (Ok(mut r), Some(s)) = (RUNNING.lock(), stdin) {
        *r = Some((path.to_owned(), s));
    }
}

pub fn ended(path: &str) {
    if let Ok(mut r) = RUNNING.lock()
        && r.as_ref().is_some_and(|(p, _)| p == path)
    {
        *r = None;
    }
}

/// Library path of the game running in RetroArch, if any.
#[tauri::command]
pub fn ra_running() -> Option<String> {
    RUNNING.lock().ok()?.as_ref().map(|(p, _)| p.clone())
}

/// Sends one command line; `false` if no game is running.
pub fn send(line: &str) -> CmdResult<bool> {
    let mut r = RUNNING.lock().map_err(err)?;
    let Some((_, stdin)) = r.as_mut() else {
        return Ok(false);
    };
    if writeln!(stdin, "{line}")
        .and_then(|_| stdin.flush())
        .is_err()
    {
        *r = None; // RetroArch is gone
        return Ok(false);
    }
    Ok(true)
}

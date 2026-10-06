//! User exceptions: verdicts on 1G1R rejects, ambiguity resolutions, ignored paths.
use crate::commands::{CmdResult, err, open_store};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct VerdictView {
    system: String,
    name: String,
    verdict: String,
    /// Why the release was up for decision (empty for old verdicts).
    reason: String,
    /// Unix time of the decision, if known.
    decided: Option<i64>,
}

#[derive(Serialize)]
pub struct ResolutionView {
    /// Hex SHA1 of the file; key for clearing.
    sha1: String,
    system: String,
    name: String,
}

#[derive(Serialize)]
pub struct Exceptions {
    verdicts: Vec<VerdictView>,
    resolutions: Vec<ResolutionView>,
    ignored: Vec<String>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> CmdResult<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| {
            s.get(i..i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("bad sha1: {s}"))
        })
        .collect()
}

#[tauri::command]
pub async fn exceptions_get() -> CmdResult<Exceptions> {
    let (store, _) = open_store()?;
    let verdicts: Vec<VerdictView> = store
        .verdict_rows()
        .map_err(err)?
        .into_iter()
        .map(|r| VerdictView {
            system: r.system,
            name: r.name,
            verdict: r.verdict,
            reason: r.reason,
            decided: r.decided,
        })
        .collect();
    let resolutions = store
        .resolutions()
        .map_err(err)?
        .into_iter()
        .map(|(sha1, system, name)| ResolutionView {
            sha1: hex(&sha1),
            system,
            name,
        })
        .collect();
    let ignored = store
        .ignored()
        .map_err(err)?
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    Ok(Exceptions {
        verdicts,
        resolutions,
        ignored,
    })
}

/// Replaces the list of paths the planner never touches.
#[tauri::command]
pub async fn ignore_set(paths: Vec<PathBuf>) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store.set_ignored(&paths).map_err(err)
}

#[tauri::command]
pub async fn resolution_clear(sha1: String) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store.clear_resolution(&unhex(&sha1)?).map_err(err)
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_roundtrip() {
        let b = [0u8, 0xab, 0x10];
        assert_eq!(super::unhex(&super::hex(&b)).unwrap(), b);
    }
}

//! File ▸ Send to Gesso: hand the current frame or the active sequence to Gesso (an app based on
//! ArtCraft, in the Gesso craft-suite) through its inbox folder (`~/Artcraft/inbox`, or the `dir`
//! parameter). The folder keeps its old name so existing Gesso installs keep reading it.
//!
//! Gesso imports every complete file in the inbox and moves it away. It ignores hidden files
//! (names starting with `.`) and names ending in `.part` / `.tmp`, and only takes files whose
//! modification time is at least 2 s old. So a send never lets a half-written file show up:
//!
//! 1. the file is written under a hidden, unique name in the inbox itself
//!    (`.FilmCraft - <name>.<pid>-<n>.<ext>`, same folder so the final step never crosses file
//!    systems);
//! 2. when it is complete it gets its final name, `FilmCraft - <name>.<ext>`, or
//!    `FilmCraft - <name> 2.<ext>`, ` 3`… when that exists. The final name is linked without
//!    replacing anything (a hard link fails when the name exists), then the hidden name is removed;
//!    where the file system has no hard links it is renamed after checking the name is free;
//! 3. a failed or cancelled send deletes the hidden file.
//!
//! - `file.sendFrameToArtcraft` renders the frame with `file.exportFrame` (PNG, Program monitor
//!   unless `target: "source"`; the UI passes the focused monitor like Export Frame does) and
//!   renames it at once.
//! - `file.sendSequenceToArtcraft` exports the active (or `sequence`) sequence, entire range, as
//!   H.264 MP4 with audio through `file.exportMedia`, as an ordinary background job. [`poll`] runs
//!   from [`Session::poll_persistence`] (every UI frame) and [`Session::pump_jobs`]: it renames the
//!   file when the job succeeds and deletes it when the job fails or is cancelled. `wait: true`
//!   exports and renames before returning.
//!
//! The web build has no inbox folder to share with Gesso: both commands are disabled there.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use crate::commands::{CommandSpec, bad, bool_p, has_seq, str_p};
use crate::{EngineError, Result, Session};

/// The inbox under the home directory.
pub const INBOX: &str = "Artcraft/inbox";
/// Final file names start with this, so Gesso can say where a file came from.
pub const PREFIX: &str = "FilmCraft - ";
/// Finished sends kept for the frontend to show ([`Sends::take_finished`]); older ones are dropped.
const MAX_FINISHED: usize = 32;
/// Longest `<name>` part of a file name, in characters.
const MAX_NAME_CHARS: usize = 120;
/// Suffixes tried before giving up on a free final name.
const MAX_SUFFIX: u32 = 9999;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Sends in progress and finished ones the frontend has not shown yet (session state).
#[derive(Default)]
pub struct Sends {
    pub pending: Vec<PendingSend>,
    finished: Vec<SendOutcome>,
}

/// A sequence export writing to a hidden file in the inbox.
#[derive(Clone, Debug)]
pub struct PendingSend {
    pub job: u64,
    pub partial: PathBuf,
    pub dir: PathBuf,
    /// The `<name>` part of the final file name (already file-safe).
    pub name: String,
    pub ext: String,
}

/// How a send ended: the final path, or the error (`cancelled` for a cancelled job).
#[derive(Clone, Debug, PartialEq)]
pub struct SendOutcome {
    pub job: Option<u64>,
    pub result: std::result::Result<String, String>,
}

impl SendOutcome {
    /// The status-bar message for this outcome.
    pub fn message(&self) -> String {
        match &self.result {
            Ok(path) => format!("Sent to Gesso: {}", file_name(Path::new(path))),
            Err(e) if e == "cancelled" => "Send to Gesso cancelled".into(),
            Err(e) => format!("Send to Gesso failed: {e}"),
        }
    }
}

impl Sends {
    /// Finished sends since the last call, oldest first.
    pub fn take_finished(&mut self) -> Vec<SendOutcome> {
        std::mem::take(&mut self.finished)
    }

    fn finish(&mut self, outcome: SendOutcome) {
        if self.finished.len() >= MAX_FINISHED {
            self.finished.remove(0);
        }
        self.finished.push(outcome);
    }
}

pub(crate) fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "file.sendFrameToArtcraft",
            label: "Current Frame",
            menu: &["File", "Send to Gesso"],
            shortcut: None,
            params: r#"{"dir":str?,"target":"source|program"?,"item":id?,"sequence":id?,"time":ticks?}"#,
            enabled: can_send,
            run: send_frame,
            journal: true,
        },
        CommandSpec {
            id: "file.sendSequenceToArtcraft",
            label: "Active Sequence",
            menu: &["File", "Send to Gesso"],
            shortcut: None,
            params: r#"{"dir":str?,"sequence":id?,"wait":bool=false}"#,
            enabled: can_send_sequence,
            run: send_sequence,
            journal: true,
        },
    ]
}

/// Put the commands right after File ▸ Export's first item: a submenu sits where its first item
/// is, so File ▸ Send to Gesso comes right after File ▸ Export.
pub(crate) fn apply_layout(v: &mut Vec<CommandSpec>) {
    let at = v.iter().position(|c| c.menu == ["File", "Export"]).map_or(v.len(), |i| i + 1);
    v.splice(at..at, commands());
}

/// Settle finished sequence sends: rename the file on success, delete it on failure or cancel.
pub fn poll(s: &mut Session) {
    if s.artcraft.pending.is_empty() {
        return;
    }
    let mut i = 0;
    while let Some(p) = s.artcraft.pending.get(i) {
        let result = match s.jobs.iter().find(|j| j.id == p.job) {
            Some(j) => j.result.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            None => Some(Err("the export job is gone".into())),
        };
        let Some(result) = result else {
            i += 1;
            continue;
        };
        let p = s.artcraft.pending.remove(i);
        let outcome = match result {
            Ok(_) => finalize(&p.partial, &p.dir, &p.name, &p.ext).map(|f| f.to_string_lossy().into_owned()),
            Err(e) => {
                discard(&p.partial);
                Err(e)
            }
        };
        report(s, Some(p.job), outcome);
    }
}

// ---------------------------------------------------------------------------------------------
// commands
// ---------------------------------------------------------------------------------------------

fn send_frame(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "file.sendFrameToArtcraft";
    can_send(s).map_err(EngineError::Other)?;
    let dir = inbox(s, p, CMD)?;
    let target = crate::keyboard::frame_export_target(s, p)?;
    let name = file_safe(&target.name);
    let partial = partial_path(&dir, &name, "png");
    let mut params = json!({"path": partial.to_string_lossy(), "format": "png", "depth": 8, "import": false});
    for k in ["target", "item", "sequence", "time"] {
        if let Some(v) = p.get(k) {
            params[k] = v.clone();
        }
    }
    let exported = match s.execute("file.exportFrame", params) {
        Ok(v) => v,
        Err(e) => {
            discard(&partial);
            return Err(e);
        }
    };
    let outcome = finalize(&partial, &dir, &name, "png").map(|f| f.to_string_lossy().into_owned());
    report(s, None, outcome.clone());
    let path = outcome.map_err(|e| EngineError::Other(format!("Send to Gesso failed: {e}")))?;
    Ok(json!({"path": path, "width": exported["width"], "height": exported["height"], "target": exported["target"], "time": exported["time"]}))
}

fn send_sequence(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "file.sendSequenceToArtcraft";
    can_send(s).map_err(EngineError::Other)?;
    let dir = inbox(s, p, CMD)?;
    let seq = crate::export_tools::sequence_param(s, p, CMD)?;
    let wait = match p.get("wait") {
        None | Some(Value::Null) => false,
        Some(_) => bool_p(p, "wait").ok_or_else(|| bad(CMD, "wait must be true or false"))?,
    };
    let name = file_safe(&s.project.item(seq).map(|i| i.name.clone()).unwrap_or_default());
    let ext = filmcraft_export::Format::H264.extension();
    let partial = partial_path(&dir, &name, ext);
    let params = json!({
        "path": partial.to_string_lossy(),
        "format": "h264",
        "audio": true,
        "range": "entire",
        "sequence": seq.0,
        "wait": wait,
    });
    let started = match s.execute("file.exportMedia", params) {
        Ok(v) => v,
        Err(e) => {
            discard(&partial);
            return Err(e);
        }
    };
    let job = started["job"].as_u64().ok_or_else(|| EngineError::Other("the export did not start a job".into()))?;
    // the export may have adjusted the path (it never does for a path ending in .mp4)
    let partial = started["path"].as_str().map_or(partial, PathBuf::from);
    s.artcraft.pending.push(PendingSend { job, partial: partial.clone(), dir: dir.clone(), name: name.clone(), ext: ext.into() });
    if wait {
        poll(s);
        let outcome = s.artcraft.finished.iter().rev().find(|o| o.job == Some(job)).map(|o| o.result.clone());
        let path = match outcome {
            Some(Ok(path)) => path,
            Some(Err(e)) => return Err(EngineError::Other(format!("Send to Gesso failed: {e}"))),
            None => return Err(EngineError::Other("Send to Gesso: the export has not finished".into())),
        };
        return Ok(json!({"job": job, "path": path, "finished": true}));
    }
    s.toast(format!("Sending {name} to Gesso…"));
    Ok(json!({"job": job, "name": name, "partial": partial.to_string_lossy(), "dir": dir.to_string_lossy(), "finished": false}))
}

fn can_send(_: &Session) -> std::result::Result<(), String> {
    if cfg!(target_arch = "wasm32") { Err("Send to Gesso needs the desktop app (Gesso reads a folder on this computer)".into()) } else { Ok(()) }
}

fn can_send_sequence(s: &Session) -> std::result::Result<(), String> {
    can_send(s)?;
    has_seq(s)
}

// ---------------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------------

/// The inbox folder (`dir`, else `<home>/Artcraft/inbox`), created when missing.
fn inbox(s: &Session, p: &Value, cmd: &str) -> Result<PathBuf> {
    let dir = match p.get("dir") {
        None | Some(Value::Null) => {
            let home = s.services.home_dir().or_else(crate::media_browser::std_home_dir).ok_or_else(|| bad(cmd, "no home folder; pass `dir`"))?;
            Path::new(&home).join(INBOX)
        }
        Some(_) => {
            let d = str_p(p, "dir").map(str::trim).filter(|d| !d.is_empty()).ok_or_else(|| bad(cmd, "dir must be a non-empty string"))?;
            PathBuf::from(crate::export_tools::expand_home(d))
        }
    };
    std::fs::create_dir_all(&dir).map_err(|e| EngineError::Other(format!("cannot create the Gesso inbox {}: {e}", dir.display())))?;
    if !dir.is_dir() {
        return Err(EngineError::Other(format!("the Gesso inbox {} is not a folder", dir.display())));
    }
    Ok(dir)
}

/// A hidden name in `dir` that nothing else uses: Gesso skips it while it is written.
fn partial_path(dir: &Path, name: &str, ext: &str) -> PathBuf {
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let p = dir.join(format!(".{PREFIX}{name}.{}-{n}.{ext}", std::process::id()));
        if !p.exists() {
            return p;
        }
    }
}

/// Give the complete file at `partial` its final name in `dir`, never replacing a file.
fn finalize(partial: &Path, dir: &Path, name: &str, ext: &str) -> std::result::Result<PathBuf, String> {
    if !partial.is_file() {
        return Err(format!("{} was not written", partial.display()));
    }
    for n in 1..=MAX_SUFFIX {
        let candidate = dir.join(final_name(name, ext, n));
        match std::fs::hard_link(partial, &candidate) {
            Ok(()) => {
                let _ = std::fs::remove_file(partial);
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            // no hard links here (FAT / exFAT, some network shares): rename onto a free name
            Err(_) if candidate.exists() => continue,
            Err(_) => {
                return match std::fs::rename(partial, &candidate) {
                    Ok(()) => Ok(candidate),
                    Err(e) => {
                        discard(partial);
                        Err(format!("{}: {e}", candidate.display()))
                    }
                };
            }
        }
    }
    discard(partial);
    Err(format!("every name for {PREFIX}{name}.{ext} is taken in {}", dir.display()))
}

/// `FilmCraft - <name>.<ext>`, then `FilmCraft - <name> 2.<ext>`, ` 3`…
pub fn final_name(name: &str, ext: &str, n: u32) -> String {
    if n <= 1 { format!("{PREFIX}{name}.{ext}") } else { format!("{PREFIX}{name} {n}.{ext}") }
}

fn report(s: &mut Session, job: Option<u64>, result: std::result::Result<String, String>) {
    let outcome = SendOutcome { job, result };
    let message = outcome.message();
    if outcome.result.is_ok() {
        s.toast(message);
    } else {
        s.error_toast("artcraft", message);
    }
    s.artcraft.finish(outcome);
}

fn discard(partial: &Path) {
    if let Err(e) = std::fs::remove_file(partial)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("{}: {e}", partial.display());
    }
}

/// A name safe on every desktop file system: no separators, reserved or control characters, no
/// leading dot (that would hide the file from Gesso), at most [`MAX_NAME_CHARS`] characters.
fn file_safe(n: &str) -> String {
    let s: String = n.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c }).take(MAX_NAME_CHARS).collect();
    let s = s.trim().trim_start_matches('.').trim_end_matches(['.', ' ']).trim();
    if s.is_empty() { "Untitled".into() } else { s.to_string() }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string_lossy().into_owned())
}

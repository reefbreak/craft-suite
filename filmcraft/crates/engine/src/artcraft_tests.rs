//! File ▸ Send to ArtCraft: the frame and the sequence land in the inbox under their final
//! `FilmCraft - <name>` names, never as a visible partial file, without overwriting earlier sends.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use filmcraft_media::{DemoScene, Generator, generators::GeneratorSource};
use filmcraft_project::Label;
use filmcraft_time::{FrameRate, TICKS_PER_SECOND, Tick};
use serde_json::json;

use crate::Session;
use crate::artcraft::PREFIX;

#[test]
fn frame_lands_in_the_inbox_under_its_final_name() {
    let inbox = Scratch::new("frame");
    let mut s = session();
    let r = s.execute("file.sendFrameToArtcraft", json!({"dir": inbox.dir()})).unwrap();
    let path = r["path"].as_str().unwrap();
    assert_eq!(r["target"], "program");
    assert_eq!(files_in(&inbox.0), [format!("{PREFIX}Sequence 01.png")], "{r}");
    assert_eq!(Path::new(path), inbox.0.join(format!("{PREFIX}Sequence 01.png")));
    let img = image::load_from_memory(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!((img.width(), img.height()), (128, 72));
    // a second send never overwrites the first
    s.execute("file.sendFrameToArtcraft", json!({"dir": inbox.dir(), "target": "source"})).unwrap();
    s.execute("file.sendFrameToArtcraft", json!({"dir": inbox.dir()})).unwrap();
    assert_eq!(files_in(&inbox.0), [format!("{PREFIX}Sequence 01 2.png"), format!("{PREFIX}Sequence 01.png"), format!("{PREFIX}Source clip.png")]);
    let finished = s.artcraft.take_finished();
    assert_eq!(finished.len(), 3);
    assert!(finished.iter().all(|o| o.result.is_ok() && o.message().starts_with("Sent to ArtCraft")));
}

#[test]
fn sequence_waits_and_lands_as_mp4() {
    let inbox = Scratch::new("sequence-wait");
    let mut s = session();
    let r = s.execute("file.sendSequenceToArtcraft", json!({"dir": inbox.dir(), "wait": true})).unwrap();
    assert_eq!(r["finished"], true);
    assert_eq!(files_in(&inbox.0), [format!("{PREFIX}Sequence 01.mp4")], "{r}");
    assert_mp4(&inbox.0.join(format!("{PREFIX}Sequence 01.mp4")));
    assert!(s.artcraft.pending.is_empty());
}

#[test]
fn background_sequence_send_is_renamed_when_the_job_finishes() {
    let inbox = Scratch::new("sequence-bg");
    let mut s = session();
    // a file already named like the send stays untouched
    std::fs::write(inbox.0.join(format!("{PREFIX}Sequence 01.mp4")), b"earlier").unwrap();
    let r = s.execute("file.sendSequenceToArtcraft", json!({"dir": inbox.dir()})).unwrap();
    assert_eq!(r["finished"], false);
    let partial = r["partial"].as_str().unwrap();
    assert!(Path::new(partial).file_name().unwrap().to_string_lossy().starts_with('.'), "written under a hidden name: {partial}");
    poll_until_settled(&mut s);
    assert_eq!(files_in(&inbox.0), [format!("{PREFIX}Sequence 01 2.mp4"), format!("{PREFIX}Sequence 01.mp4")]);
    assert_eq!(std::fs::read(inbox.0.join(format!("{PREFIX}Sequence 01.mp4"))).unwrap(), b"earlier");
    assert_mp4(&inbox.0.join(format!("{PREFIX}Sequence 01 2.mp4")));
    let finished = s.artcraft.take_finished();
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].job, r["job"].as_u64());
    assert_eq!(finished[0].message(), format!("Sent to ArtCraft: {PREFIX}Sequence 01 2.mp4"));
}

#[test]
fn cancelled_sequence_send_leaves_nothing_behind() {
    let inbox = Scratch::new("sequence-cancel");
    let mut s = session();
    let r = s.execute("file.sendSequenceToArtcraft", json!({"dir": inbox.dir()})).unwrap();
    s.execute("jobs.cancel", json!({"job": r["job"]})).unwrap();
    poll_until_settled(&mut s);
    let finished = s.artcraft.take_finished();
    assert_eq!(finished.len(), 1);
    // the job may have finished before the cancel arrived; either way no partial file is left
    match &finished[0].result {
        Ok(_) => assert_eq!(files_in(&inbox.0), [format!("{PREFIX}Sequence 01.mp4")]),
        Err(_) => assert!(files_in(&inbox.0).is_empty(), "{:?}", files_in(&inbox.0)),
    }
}

#[test]
fn inbox_is_created_and_hostile_parameters_are_errors() {
    let inbox = Scratch::new("hostile");
    let nested = inbox.0.join("a").join("inbox");
    let mut s = session();
    s.execute("file.sendFrameToArtcraft", json!({"dir": nested.to_string_lossy()})).unwrap();
    assert_eq!(files_in(&nested), [format!("{PREFIX}Sequence 01.png")]);
    for params in [
        json!({"dir": ""}),
        json!({"dir": 7}),
        json!({"dir": inbox.dir(), "target": "nowhere"}),
        json!({"dir": inbox.dir(), "time": -1}),
        json!({"dir": inbox.dir(), "sequence": u64::MAX}),
    ] {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.execute("file.sendFrameToArtcraft", params.clone())));
        assert!(r.expect("no panic").is_err(), "{params}");
    }
    for params in [json!({"dir": inbox.dir(), "wait": "yes"}), json!({"dir": inbox.dir(), "sequence": u64::MAX}), json!({"dir": [1]})] {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.execute("file.sendSequenceToArtcraft", params.clone())));
        assert!(r.expect("no panic").is_err(), "{params}");
    }
    assert!(files_in(&inbox.0).iter().all(|f| f == "a"), "failed sends leave no files: {:?}", files_in(&inbox.0));
    // a file where the inbox should be
    let file = inbox.0.join("not-a-folder");
    std::fs::write(&file, b"x").unwrap();
    assert!(s.execute("file.sendFrameToArtcraft", json!({"dir": file.to_string_lossy()})).is_err());
}

#[test]
fn sequence_send_needs_a_sequence_and_both_are_in_the_file_menu() {
    let s = Session::default();
    assert!(!s.is_enabled("file.sendSequenceToArtcraft"));
    assert!(s.is_enabled("file.sendFrameToArtcraft"), "enabled; it reports why it cannot run");
    for id in ["file.sendFrameToArtcraft", "file.sendSequenceToArtcraft"] {
        assert_eq!(crate::find_command(id).unwrap().menu, ["File", "Send to ArtCraft"]);
    }
    // File ▸ Send to ArtCraft follows the first File ▸ Export item
    let specs = crate::command_specs();
    let export = specs.iter().position(|c| c.menu == ["File", "Export"]).unwrap();
    assert_eq!(specs[export + 1].id, "file.sendFrameToArtcraft");
    assert_eq!(specs[export + 2].id, "file.sendSequenceToArtcraft");
}

#[test]
fn final_names_count_up_and_names_are_file_safe() {
    assert_eq!(crate::artcraft::final_name("Cut", "png", 1), "FilmCraft - Cut.png");
    assert_eq!(crate::artcraft::final_name("Cut", "png", 3), "FilmCraft - Cut 3.png");
    let inbox = Scratch::new("names");
    let mut s = session();
    let seq = s.state.active_sequence.unwrap();
    Arc::make_mut(&mut s.project).item_mut(seq).unwrap().name = "../.hidden: a/b?".into();
    let r = s.execute("file.sendFrameToArtcraft", json!({"dir": inbox.dir()})).unwrap();
    let name = Path::new(r["path"].as_str().unwrap()).file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(name, format!("{PREFIX}_.hidden_ a_b_.png"));
    assert_eq!(files_in(&inbox.0), [name]);
}

// ---------------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------------

/// A 128×72, 24 fps, 2 s sequence with one generated clip (also open in the Source monitor).
fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.newSequence", json!({"width": 128, "height": 72, "fps": 24})).unwrap();
    let item = crate::demo::add_generator(
        Arc::make_mut(&mut s.project),
        &s.media,
        GeneratorSource::new(Generator::Demo(DemoScene::Aurora), 128, 72, FrameRate::FPS_24, Tick(2 * TICKS_PER_SECOND)),
        "Source clip",
        Label::Iris,
        None,
    );
    s.execute("timeline.place", json!({"item": item.0, "track": "V1"})).unwrap();
    s.execute("source.open", json!({"item": item.0})).unwrap();
    s
}

/// Run the per-frame hook until no send is pending (at most two minutes).
fn poll_until_settled(s: &mut Session) {
    let t0 = Instant::now();
    while !s.artcraft.pending.is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(120), "the send did not finish");
        std::thread::sleep(Duration::from_millis(20));
        s.poll_persistence();
    }
}

/// An MP4 file (`ftyp` box) with an H.264 video track; ffprobe also checks for the audio track.
fn assert_mp4(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() > 1000 && bytes.get(4..8) == Some(b"ftyp".as_slice()), "{} is not an MP4", path.display());
    let Some(ffprobe) = filmcraft_testkit::ffprobe_or_skip("Send to ArtCraft") else { return };
    let out = std::process::Command::new(ffprobe).args(["-v", "error", "-show_entries", "stream=codec_name", "-of", "csv=p=0"]).arg(path).output().unwrap();
    let codecs = String::from_utf8_lossy(&out.stdout);
    assert!(codecs.contains("h264") && codecs.contains("aac"), "{codecs}");
}

/// Names in `dir`, sorted (hidden files included, so a leftover partial file shows up).
fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// A scratch inbox under the workspace `target/`, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let d = filmcraft_testkit::workspace_root().join("target").join("artcraft-tests").join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }
    fn dir(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

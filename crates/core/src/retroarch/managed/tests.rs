use super::config::merge;
use super::*;
use std::cell::RefCell;

fn linux(root: &Path) -> Managed {
    Managed {
        root: root.to_path_buf(),
        target: Target::LinuxX64,
        display: None,
    }
}

/// A `.7z` shaped like the Linux build, with `body` as the AppImage.
fn fake_archive(dir: &Path, body: &[u8]) -> PathBuf {
    let src = dir.join("src");
    let exe = src.join(Target::LinuxX64.executable());
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::write(&exe, body).unwrap();
    let out = dir.join("RetroArch.7z");
    sevenz_rust2::compress_to_path(&src, &out).unwrap();
    out
}

#[allow(clippy::type_complexity)]
fn copy_fetch(
    from: PathBuf,
    urls: &RefCell<Vec<String>>,
) -> impl Fn(&str, &Path, &dyn Fn(u64, Option<u64>)) -> io::Result<()> {
    move |url, to, progress| {
        urls.borrow_mut().push(url.to_owned());
        let n = std::fs::copy(&from, to)?;
        progress(n, Some(n));
        Ok(())
    }
}

#[test]
fn installs_switches_and_prunes() {
    let t = tempfile::tempdir().unwrap();
    let archive = fake_archive(t.path(), b"#!/bin/sh\n");
    let m = linux(&t.path().join("ra"));
    let urls = RefCell::new(Vec::new());
    let fetch = copy_fetch(archive, &urls);
    let phases = RefCell::new(Vec::new());
    m.install("9.9.0", &fetch, &|p| phases.borrow_mut().push(p))
        .unwrap();
    assert_eq!(m.current().as_deref(), Some("9.9.0"));
    assert!(m.executable().unwrap().is_file());
    assert_eq!(
        urls.borrow()[0],
        "https://buildbot.libretro.com/stable/9.9.0/linux/x86_64/RetroArch.7z"
    );
    assert_eq!(phases.borrow().last(), Some(&Phase::Done));
    let unpacked = phases.borrow().iter().rev().find_map(|p| match p {
        Phase::Unpack { done, total } => Some((*done, *total)),
        _ => None,
    });
    assert_eq!(
        unpacked,
        Some((10, 10)),
        "unpack progress ends at the archive size"
    );
    assert!(!m.root.join("download").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(m.executable().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    m.install("9.9.1", &fetch, &|_| {}).unwrap();
    assert_eq!(m.current().as_deref(), Some("9.9.1"));
    assert!(!m.version_dir("9.9.0").exists(), "old version pruned");
    // Re-activating an installed version downloads nothing.
    m.install("9.9.1", &fetch, &|_| {}).unwrap();
    assert_eq!(urls.borrow().len(), 2);
}

#[test]
fn pinned_checksum_mismatch_keeps_old_install() {
    let t = tempfile::tempdir().unwrap();
    let archive = fake_archive(t.path(), b"not the real build");
    let m = linux(&t.path().join("ra"));
    let urls = RefCell::new(Vec::new());
    let fetch = copy_fetch(archive, &urls);
    m.install("9.9.0", &fetch, &|_| {}).unwrap();
    let err = m.install(PINNED, &fetch, &|_| {}).unwrap_err();
    assert!(err.to_string().contains("checksum mismatch"), "{err}");
    assert_eq!(m.current().as_deref(), Some("9.9.0"));
    assert!(!m.version_dir(PINNED).exists());
}

#[test]
fn archive_without_executable_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    let src = t.path().join("src/other");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("x"), b"x").unwrap();
    let archive = t.path().join("a.7z");
    sevenz_rust2::compress_to_path(t.path().join("src"), &archive).unwrap();
    let m = linux(&t.path().join("ra"));
    let urls = RefCell::new(Vec::new());
    let err = m
        .install("9.9.0", &copy_fetch(archive, &urls), &|_| {})
        .unwrap_err();
    assert!(err.to_string().contains("lacks"), "{err}");
    assert_eq!(m.current(), None);
}

#[test]
fn failed_download_reports_url() {
    let t = tempfile::tempdir().unwrap();
    let m = linux(t.path());
    let fetch = |_: &str, _: &Path, _: &dyn Fn(u64, Option<u64>)| Err(io::Error::other("offline"));
    let err = m.install("9.9.0", &fetch, &|_| {}).unwrap_err().to_string();
    assert!(
        err.contains("stable/9.9.0") && err.contains("offline"),
        "{err}"
    );
}

#[test]
fn latest_stable_from_listing() {
    let html = r#"<a href="/stable/1.9.0/">1.9.0</a> <a href="/stable/1.22.2/">x</a>
        <a href="/stable/1.22.10/">x</a> <a href="/stable/nightly/">x</a> <a href="/">up</a>"#;
    assert_eq!(latest_stable(html).as_deref(), Some("1.22.10"));
    assert_eq!(latest_stable("no links"), None);
}

#[test]
fn config_keeps_user_lines_and_sets_folders() {
    let old = "video_fullscreen = \"true\"\nsystem_directory = \"/old\"\n";
    let out = merge(old, &[("system_directory", "/lib/_bios".into())]);
    assert_eq!(
        out,
        "video_fullscreen = \"true\"\nsystem_directory = \"/lib/_bios\"\n"
    );

    let t = tempfile::tempdir().unwrap();
    let m = linux(t.path());
    assert!(m.write_config(None).is_err(), "needs an installed version");
    let exe = m.executable_of("9.9.0");
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::write(&exe, b"").unwrap();
    std::fs::write(t.path().join("current"), "9.9.0").unwrap();
    std::fs::write(m.cfg(), old).unwrap();
    let lib = t.path().join("lib");
    m.write_config(Some(&lib)).unwrap();
    let cfg = std::fs::read_to_string(m.cfg()).unwrap();
    assert!(cfg.contains("video_fullscreen = \"true\""));
    assert!(cfg.contains(&format!(
        "system_directory = \"{}\"",
        lib.join("_bios").display()
    )));
    assert!(cfg.contains("AppImage.home/.config/retroarch/assets"));
    assert!(t.path().join("saves").is_dir() && t.path().join("cores").is_dir());
    assert!(!lib.exists(), "library untouched");
    let m = Managed {
        display: Some(super::display::Display::Window { scale: 2 }),
        ..m
    };
    m.write_config(None).unwrap();
    let cfg = std::fs::read_to_string(m.cfg()).unwrap();
    assert!(cfg.contains("video_fullscreen = \"false\"") && cfg.contains("video_scale = \"2\""));
    assert!(!cfg.contains("video_fullscreen = \"true\""));
}

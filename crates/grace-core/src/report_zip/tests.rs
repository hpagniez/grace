// `src/main/report-zip.test.ts`, test for test (ER-08, ER-09, ER-10), and the plan's porting rule 2: a secret never
// reaches the zip.
use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A run folder under a name outside the ANSI code page: Karate's report, its JUnit XML and event file, and Grace's
/// run.json.
fn run_folder(root: &Path) -> PathBuf {
    let dir = root.join("отчёт łódź");
    let files = [
        ("karate-summary.html", "<html>"),
        ("feature-html/café.html", "<html>"),
        ("junit-xml/a.xml", "<testsuite/>"),
        ("karate-json/karate-events.jsonl", "{}"),
        ("run.json", "{}"),
    ];
    for (f, text) in files {
        let path = dir.join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    dir
}

fn names(zip: &Path) -> Vec<String> {
    let mut names: Vec<_> = read_zip(&fs::read(zip).unwrap())
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    names.sort();
    names
}

const REPORT: [&str; 3] = ["feature-html/café.html", "junit-xml/a.xml", "karate-summary.html"];

/// 1 (ER-08): the zip holds the report and leaves out run.json and karate-json.
#[test]
fn holds_the_report_and_leaves_out_run_json_and_karate_json() {
    let t = tempfile::tempdir().unwrap();
    let zip = t.path().join("r.zip");
    zip_report(&run_folder(t.path()), &zip).unwrap();
    assert_eq!(names(&zip), REPORT);
}

/// 2 (ER-09): the chosen path's characters are the file system's, not a command line's.
#[test]
fn writes_to_the_chosen_path_whatever_its_characters() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out dir 報告");
    fs::create_dir(&out).unwrap();
    let zip = out.join("r łódź.zip");
    zip_report(&run_folder(t.path()), &zip).unwrap();
    let left: Vec<_> = fs::read_dir(&out).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(left, ["r łódź.zip"]);
    assert_eq!(names(&zip), REPORT);
}

/// 3: a zip whatever the name the person typed.
#[test]
fn writes_a_zip_whatever_the_name() {
    let t = tempfile::tempdir().unwrap();
    let zip = t.path().join("r.bin");
    zip_report(&run_folder(t.path()), &zip).unwrap();
    assert_eq!(names(&zip), REPORT);
}

/// 4: a file the person chose to replace is replaced.
#[test]
fn replaces_an_existing_file() {
    let t = tempfile::tempdir().unwrap();
    let zip = t.path().join("r.zip");
    fs::write(&zip, "junk").unwrap();
    zip_report(&run_folder(t.path()), &zip).unwrap();
    assert_eq!(names(&zip), REPORT);
}

/// 5 (ER-10): the folder is gone, so the error says why and nothing is left.
#[test]
fn leaves_nothing_when_the_folder_is_gone() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    fs::create_dir(&out).unwrap();
    let e = zip_report(&t.path().join("missing"), &out.join("r.zip")).unwrap_err();
    assert!(!e.to_string().trim().is_empty());
    assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
}

/// 6: and the file it would replace stays as it was.
#[test]
fn keeps_the_file_it_would_replace_when_it_fails() {
    let t = tempfile::tempdir().unwrap();
    let zip = t.path().join("r.zip");
    fs::write(&zip, "old").unwrap();
    zip_report(&t.path().join("missing"), &zip).unwrap_err();
    assert_eq!(fs::read_dir(t.path()).unwrap().count(), 1);
    assert_eq!(fs::read_to_string(&zip).unwrap(), "old");
}

/// ER-10: the destination cannot be written (its folder is not there): nothing is left either.
#[test]
fn leaves_nothing_when_the_destination_cannot_be_written() {
    let t = tempfile::tempdir().unwrap();
    let dir = run_folder(t.path());
    zip_report(&dir, &t.path().join("no such folder").join("r.zip")).unwrap_err();
    assert_eq!(fs::read_dir(t.path()).unwrap().count(), 1, "only the run folder");
}

/// Porting rule 2: Grace's own record and Karate's event stream, where a secret could be, are not in the zip, and nothing
/// the report holds is changed on its way in.
#[test]
fn a_secret_in_the_files_left_out_never_reaches_the_zip() {
    const SECRET: &str = "s3cret-token-123";
    let t = tempfile::tempdir().unwrap();
    let dir = run_folder(t.path());
    fs::write(dir.join("run.json"), format!(r#"{{"token":"{SECRET}"}}"#)).unwrap();
    fs::write(dir.join("karate-json/karate-events.jsonl"), SECRET).unwrap();
    let zip = t.path().join("r.zip");
    zip_report(&dir, &zip).unwrap();
    // deflated, so look in what the files hold, and in the bytes too
    for (name, data) in read_zip(&fs::read(&zip).unwrap()).unwrap() {
        assert!(!String::from_utf8_lossy(&data).contains(SECRET), "{name}");
    }
    assert!(!String::from_utf8_lossy(&fs::read(&zip).unwrap()).contains(SECRET));
}

/// What a person unzips is what Karate wrote: a big file comes back byte for byte.
#[test]
fn holds_a_big_report_whole() {
    let t = tempfile::tempdir().unwrap();
    let dir = run_folder(t.path());
    let big: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    fs::write(dir.join("feature-html/big.html"), &big).unwrap();
    let zip = t.path().join("r.zip");
    zip_report(&dir, &zip).unwrap();
    let files = read_zip(&fs::read(&zip).unwrap()).unwrap();
    let got = files.iter().find(|(n, _)| n == "feature-html/big.html").unwrap();
    assert!(got.1 == big);
    assert!((fs::metadata(&zip).unwrap().len() as usize) < big.len(), "deflated");
}

/// A zip Windows' own unzipper reads: names in UTF-8, as its `tar.exe` wrote them.
#[cfg(windows)]
#[test]
fn windows_own_tar_unpacks_it() {
    let t = tempfile::tempdir().unwrap();
    let zip = t.path().join("r.zip");
    zip_report(&run_folder(t.path()), &zip).unwrap();
    let out = t.path().join("unpacked");
    fs::create_dir(&out).unwrap();
    let tar = Path::new(&std::env::var_os("SystemRoot").unwrap()).join("System32/tar.exe");
    let status = std::process::Command::new(tar)
        .arg("-xf")
        .arg(&zip)
        .arg("-C")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        fs::read_to_string(out.join("feature-html/café.html")).unwrap(),
        "<html>"
    );
    assert_eq!(
        fs::read_to_string(out.join("junit-xml").join("a.xml")).unwrap(),
        "<testsuite/>"
    );
}

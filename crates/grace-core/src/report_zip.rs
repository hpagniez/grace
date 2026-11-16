//! `src/main/report-zip.ts`: a run's report as one zip file (docs/specs/2026-11-03-export-reports.md R3).
//!
//! Electron Grace had Windows' `tar.exe` write the zip. Here it is written in Rust, with `flate2`, which Grace.exe holds
//! already (the PNG decoder links it): no new crate, and no program to start, so no argument list that reads a path in
//! the ANSI code page, which was the reason `tar` was handed no path at all (Redesigns, `report_zip`).
use std::io::{self, Read as _, Write as _};
use std::path::Path;

/// Writes the report of the run folder `dir` to `dest` as a zip, names in UTF-8. It leaves out Grace's `run.json` and
/// Karate's event folder `karate-json`; `karate-summary.html` is at its root. The zip is written beside `dest` and
/// moved onto it, so on failure `dest` keeps what it had and nothing is left behind.
pub fn zip_report(dir: &Path, dest: &Path) -> io::Result<()> {
    let mut part = dest.as_os_str().to_owned();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    part.push(format!(".{}-{nanos}.tmp", std::process::id()));
    let part = std::path::PathBuf::from(part);
    let written = write_zip(dir, &part).and_then(|()| std::fs::rename(&part, dest));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written
}

/// Grace's record and Karate's event stream: not part of the report.
const LEFT_OUT: [&str; 2] = ["run.json", "karate-json"];

/// Every file under `dir` as `(name with "/", path)`, without what is left out at the top.
fn files(dir: &Path, prefix: &str, out: &mut Vec<(String, std::path::PathBuf)>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if prefix.is_empty() && LEFT_OUT.contains(&name.as_str()) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            files(&entry.path(), &format!("{name}/"), out)?;
        } else {
            out.push((name, entry.path()));
        }
    }
    Ok(())
}

/// The DOS date and time of a zip entry, in local time; a file older than 1980 is stamped 1980.
fn dos_time(modified: std::time::SystemTime) -> (u16, u16) {
    use chrono::{Datelike as _, Timelike as _};
    let t: chrono::DateTime<chrono::Local> = modified.into();
    let year = (t.year().clamp(1980, 2107) - 1980) as u16;
    let date = year << 9 | (t.month() as u16) << 5 | t.day() as u16;
    (
        date,
        (t.hour() as u16) << 11 | (t.minute() as u16) << 5 | (t.second() / 2) as u16,
    )
}

// shortcut: no zip64, so a report of 65,535 files or a file of 4 GiB is refused, and each file is deflated in memory;
// a Karate report is a few thousand small files, upgrade if one ever is not
fn write_zip(dir: &Path, part: &Path) -> io::Result<()> {
    let too_big = || io::Error::new(io::ErrorKind::InvalidInput, "the report is too big for a zip file");
    let mut list = Vec::new();
    files(dir, "", &mut list)?;
    list.sort();
    if list.len() > u16::MAX as usize {
        return Err(too_big());
    }
    let mut out = io::BufWriter::new(std::fs::File::create(part)?);
    let mut offset = 0u32;
    let mut central = Vec::new();
    for (name, path) in &list {
        let data = std::fs::read(path)?;
        let (date, time) = dos_time(std::fs::metadata(path)?.modified()?);
        let mut sum = flate2::Crc::new();
        sum.update(&data);
        let mut packer = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        packer.write_all(&data)?;
        let packed = packer.finish()?;
        let (size, packed_size) = (
            u32::try_from(data.len()).map_err(|_| too_big())?,
            u32::try_from(packed.len()).map_err(|_| too_big())?,
        );
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;
        // the fields the local header and the central entry share: version 2.0, flag bit 11 (UTF-8 names), deflate
        let mut common = Vec::new();
        common.extend_from_slice(&20u16.to_le_bytes());
        common.extend_from_slice(&0x0800u16.to_le_bytes());
        common.extend_from_slice(&8u16.to_le_bytes());
        common.extend_from_slice(&time.to_le_bytes());
        common.extend_from_slice(&date.to_le_bytes());
        common.extend_from_slice(&sum.sum().to_le_bytes());
        common.extend_from_slice(&packed_size.to_le_bytes());
        common.extend_from_slice(&size.to_le_bytes());
        common.extend_from_slice(&name_len.to_le_bytes());
        common.extend_from_slice(&0u16.to_le_bytes()); // no extra field

        let mut local = b"PK\x03\x04".to_vec();
        local.extend_from_slice(&common);
        local.extend_from_slice(name.as_bytes());
        out.write_all(&local)?;
        out.write_all(&packed)?;

        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&common);
        central.extend_from_slice(&[0; 10]); // no comment, disk 0, no attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
        offset = offset
            .checked_add(local.len() as u32)
            .and_then(|o| o.checked_add(packed_size))
            .ok_or_else(too_big)?;
    }
    let count = (list.len() as u16).to_le_bytes();
    out.write_all(&central)?;
    out.write_all(b"PK\x05\x06\0\0\0\0")?;
    out.write_all(&count)?;
    out.write_all(&count)?;
    out.write_all(&u32::try_from(central.len()).map_err(|_| too_big())?.to_le_bytes())?;
    out.write_all(&offset.to_le_bytes())?;
    out.write_all(&0u16.to_le_bytes())?;
    out.flush()
}

/// The files of a zip written by [`zip_report`], `(name, content)` in the order they are in. For tests, here and in
/// `grace-e2e`: it reads only the stored and deflated entries of one zip, whole.
pub fn read_zip(zip: &[u8]) -> io::Result<Vec<(String, Vec<u8>)>> {
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
    let u16_at = |at: usize| zip.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |at: usize| {
        zip.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let end = (0..zip.len().saturating_sub(21))
        .rev()
        .find(|&i| u32_at(i) == Some(0x0605_4b50))
        .ok_or_else(|| bad("no end of central directory"))?;
    let (count, mut at) = (
        u16_at(end + 10).ok_or_else(|| bad("short"))?,
        u32_at(end + 16).ok_or_else(|| bad("short"))?,
    );
    let mut files = Vec::new();
    for _ in 0..count {
        if u32_at(at) != Some(0x0201_4b50) {
            return Err(bad("no central directory entry"));
        }
        let field = |o: usize| u32_at(at + o).ok_or_else(|| bad("short"));
        let small = |o: usize| u16_at(at + o).ok_or_else(|| bad("short"));
        let (method, crc, packed, size) = (small(10)?, field(16)?, field(20)?, field(24)?);
        let (name, extra, comment, local) = (small(28)?, small(30)?, small(32)?, field(42)?);
        let name_text = zip
            .get(at + 46..at + 46 + name)
            .and_then(|n| std::str::from_utf8(n).ok())
            .ok_or_else(|| bad("a name that is not UTF-8"))?
            .to_string();
        let start = local
            + 30
            + u16_at(local + 26).ok_or_else(|| bad("short"))?
            + u16_at(local + 28).ok_or_else(|| bad("short"))?;
        let packed = zip.get(start..start + packed).ok_or_else(|| bad("short data"))?;
        let mut data = Vec::with_capacity(size);
        match method {
            0 => data.extend_from_slice(packed),
            8 => {
                flate2::read::DeflateDecoder::new(packed).read_to_end(&mut data)?;
            }
            _ => return Err(bad("a method that is neither stored nor deflated")),
        }
        let mut sum = flate2::Crc::new();
        sum.update(&data);
        if data.len() != size || sum.sum() as usize != crc {
            return Err(bad("a file that does not match its sum"));
        }
        files.push((name_text, data));
        at += 46 + name + extra + comment;
    }
    Ok(files)
}

#[cfg(test)]
mod tests;

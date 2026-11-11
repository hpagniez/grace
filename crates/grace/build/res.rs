//! Grace's icon as a Windows resource file (`.res`), which `build.rs` hands the linker to put in `Grace.exe`: the
//! repository's `assets/icon.ico`, the one Electron Grace's exe carries. GPUI's Windows backend loads icon 1 of the exe
//! for the window, its title bar and its taskbar button, and Explorer shows the exe's first icon.
//!
//! The file is what `rc.exe` makes of `1 ICON "icon.ico"`, written here so that the build needs no resource compiler
//! and no crate. An `.ico` is a directory of images: the `.res` holds each image as an `RT_ICON` resource, numbered
//! from 1, then the directory as the `RT_GROUP_ICON` resource 1, each entry naming its image by that number.

/// A resource's type: one image of an icon.
pub const RT_ICON: u16 = 3;
/// A resource's type: an icon's directory of images.
pub const RT_GROUP_ICON: u16 = 14;
/// US English, the language `rc.exe` gives a resource by default.
const LANGUAGE: u16 = 0x0409;
/// The memory flags `rc.exe` writes: moveable and discardable, and pure for the directory.
const IMAGE_FLAGS: u16 = 0x1010;
const GROUP_FLAGS: u16 = 0x1030;

/// The `.res` file holding `ico` as icon 1, or what is wrong with `ico`.
pub fn icon(ico: &[u8]) -> Result<Vec<u8>, String> {
    let bytes = |at: usize, len: usize| {
        ico.get(at..at + len)
            .ok_or_else(|| format!("the icon file ends at byte {}, before byte {}", ico.len(), at + len))
    };
    let word = |at: usize| bytes(at, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let dword = |at: usize| bytes(at, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    if word(0)? != 0 || word(2)? != 1 {
        return Err("not an icon file: its header is not 0, 1".into());
    }
    let count = word(4)?;
    if count == 0 {
        return Err("the icon file holds no image".into());
    }
    // the empty resource every .res file starts with
    let mut res = Vec::new();
    resource(&mut res, 0, 0, 0, 0, &[]);
    let mut group = Vec::with_capacity(6 + 14 * usize::from(count));
    group.extend_from_slice(&[0, 0, 1, 0]);
    group.extend_from_slice(&count.to_le_bytes());
    for i in 0..count {
        let id = i + 1;
        let entry = 6 + 16 * usize::from(i);
        let size = dword(entry + 8)?;
        let image = bytes(dword(entry + 12)? as usize, size as usize)?;
        resource(&mut res, RT_ICON, id, IMAGE_FLAGS, LANGUAGE, image);
        // the entry as the file has it, its image's offset in the file replaced by its number
        group.extend_from_slice(bytes(entry, 12)?);
        group.extend_from_slice(&id.to_le_bytes());
    }
    resource(&mut res, RT_GROUP_ICON, 1, GROUP_FLAGS, LANGUAGE, &group);
    Ok(res)
}

/// Appends one resource with a numbered type and name: its 32-byte header, then its data padded to 4 bytes.
fn resource(res: &mut Vec<u8>, kind: u16, name: u16, flags: u16, language: u16, data: &[u8]) {
    let size = u32::try_from(data.len()).expect("an image under 4 GB");
    res.extend_from_slice(&size.to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes());
    for id in [kind, name] {
        res.extend_from_slice(&0xffffu16.to_le_bytes());
        res.extend_from_slice(&id.to_le_bytes());
    }
    // DataVersion, then MemoryFlags and LanguageId, then Version and Characteristics
    res.extend_from_slice(&0u32.to_le_bytes());
    res.extend_from_slice(&flags.to_le_bytes());
    res.extend_from_slice(&language.to_le_bytes());
    res.extend_from_slice(&[0; 8]);
    res.extend_from_slice(data);
    res.resize(res.len().next_multiple_of(4), 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One resource read back: its type, name, flags, language and data.
    #[derive(Debug, PartialEq)]
    struct Entry {
        kind: u16,
        name: u16,
        flags: u16,
        language: u16,
        data: Vec<u8>,
    }

    fn read(res: &[u8]) -> Vec<Entry> {
        let word = |at: usize| u16::from_le_bytes([res[at], res[at + 1]]);
        let dword = |at: usize| u32::from_le_bytes([res[at], res[at + 1], res[at + 2], res[at + 3]]) as usize;
        let mut at = 0;
        let mut entries = Vec::new();
        while at < res.len() {
            assert_eq!(at % 4, 0, "each resource starts on 4 bytes");
            let (size, header) = (dword(at), dword(at + 4));
            assert_eq!(header, 32, "a numbered type and name make a 32-byte header");
            assert_eq!(
                (word(at + 8), word(at + 12)),
                (0xffff, 0xffff),
                "the type and the name are numbers"
            );
            entries.push(Entry {
                kind: word(at + 10),
                name: word(at + 14),
                flags: word(at + 20),
                language: word(at + 22),
                data: res[at + 32..at + 32 + size].to_vec(),
            });
            at = (at + 32 + size).next_multiple_of(4);
        }
        assert_eq!(at, res.len(), "the file ends with its last resource");
        entries
    }

    fn graces_icon() -> Vec<u8> {
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icon.ico")).unwrap()
    }

    #[test]
    fn holds_each_image_of_graces_icon_then_its_directory_as_icon_1() {
        let ico = graces_icon();
        let entries = read(&icon(&ico).unwrap());
        let count = usize::from(u16::from_le_bytes([ico[4], ico[5]]));
        assert!(count > 1, "Grace's icon has an image for each size");
        assert_eq!(entries.len(), 1 + count + 1);
        assert_eq!(
            entries[0],
            Entry {
                kind: 0,
                name: 0,
                flags: 0,
                language: 0,
                data: vec![]
            },
            "a .res file starts with an empty resource"
        );
        let field = |at: usize| u32::from_le_bytes([ico[at], ico[at + 1], ico[at + 2], ico[at + 3]]) as usize;
        let mut group = ico[..6].to_vec();
        for i in 0..count {
            let entry = 6 + 16 * i;
            let (size, offset) = (field(entry + 8), field(entry + 12));
            let image = &entries[1 + i];
            assert_eq!((image.kind, image.name), (RT_ICON, i as u16 + 1));
            assert_eq!((image.flags, image.language), (0x1010, 0x0409));
            assert_eq!(
                image.data,
                ico[offset..offset + size],
                "image {} as the file holds it",
                i + 1
            );
            group.extend_from_slice(&ico[entry..entry + 12]);
            group.extend_from_slice(&(i as u16 + 1).to_le_bytes());
        }
        let directory = &entries[1 + count];
        assert_eq!(
            (directory.kind, directory.name),
            (RT_GROUP_ICON, 1),
            "the directory is icon 1"
        );
        assert_eq!((directory.flags, directory.language), (0x1030, 0x0409));
        assert_eq!(
            directory.data, group,
            "the file's directory, each image named by its number"
        );
    }

    #[test]
    fn refuses_a_file_that_is_not_a_whole_icon() {
        let ico = graces_icon();
        assert!(icon(b"<svg/>").unwrap_err().contains("not an icon file"));
        assert!(icon(&[0, 0, 1, 0, 0, 0]).unwrap_err().contains("no image"));
        assert!(icon(&ico[..ico.len() - 1]).unwrap_err().contains("the icon file ends"));
    }
}

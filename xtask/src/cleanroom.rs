//! `cargo xtask cleanroom`: no Adobe copyright notices and no files made with Adobe applications
//! in the repository (AGENTS.md › Clean-room).
//!
//! Every tracked or new file is read as bytes (fixtures may be binary) and each line checked for
//! * a copyright notice naming Adobe (`Copyright 2020 Adobe`, `© Adobe`, `(c) Adobe`, `Adobe
//!   Systems Incorporated`);
//! * a creator stamp of an Adobe application: a DSC `%%Creator:` naming Adobe or a `%%Title:`
//!   naming Adobe Illustrator, a PDF `/Creator (Adobe…` or `/Producer (Adobe…`, an XMP
//!   `xmp:CreatorTool` starting with Adobe.
//!
//! A line that has to keep one on purpose (a hand-written header that selects an importer's
//! dialect) says `cleanroom-ok` on that line; licence texts that must reproduce a notice verbatim
//! are listed in [`ALLOW_FILES`].

use std::path::Path;

/// Marks a line that keeps a match on purpose.
const ALLOW: &str = "cleanroom-ok";

/// Files that must quote a notice verbatim, with why.
const ALLOW_FILES: &[(&str, &str)] = &[
    ("NOTICE", "the BSD licence of the CMap resources hayro-cmap compiles in, reproduced as it requires"),
    ("assets/fonts/OFL-SourceSans3.txt", "the SIL OFL licence text of an open-source font"),
    ("assets/fonts/OFL-SourceSerif4.txt", "the SIL OFL licence text of an open-source font"),
];

/// This file spells out what it looks for.
const SELF: &str = "xtask/src/cleanroom.rs";

/// Bigger files are not read (nothing in the repository comes near it).
const MAX_BYTES: u64 = 256 << 20;

#[derive(Debug, PartialEq)]
pub struct Hit {
    pub path: String,
    /// 1-based.
    pub line: usize,
    pub what: &'static str,
    pub context: String,
}

pub fn run(root: &Path) -> Result<(), String> {
    let files = crate::repo_files(root)?;
    let hits = scan(root, &files);
    if hits.is_empty() {
        println!("cleanroom: no Adobe copyright notices or Adobe-made files ({} files)", files.len());
        return Ok(());
    }
    let list: Vec<String> = hits.iter().map(|h| format!("{}:{}: {}: {}", h.path, h.line, h.what, h.context)).collect();
    Err(format!(
        "{} clean-room violation(s): remove the file or notice (test inputs are written from the public specs); a hand-written line that must keep it says `{ALLOW}`:\n  {}",
        hits.len(),
        list.join("\n  ")
    ))
}

/// Every hit in `files` (paths relative to `root`).
pub fn scan(root: &Path, files: &[String]) -> Vec<Hit> {
    let mut hits = vec![];
    for path in files {
        if path == SELF || ALLOW_FILES.iter().any(|(p, _)| p == path) {
            continue;
        }
        let full = root.join(path);
        if std::fs::metadata(&full).map_or(true, |m| !m.is_file() || m.len() > MAX_BYTES) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&full) else { continue };
        hits.extend(scan_bytes(&bytes).into_iter().map(|(line, what, context)| Hit { path: path.clone(), line, what, context }));
    }
    hits
}

/// (line, what, line text) for each match in `bytes`: lines end at LF, or at CR in a file
/// without LFs; a CR inside a line still starts a DSC comment.
fn scan_bytes(bytes: &[u8]) -> Vec<(usize, &'static str, String)> {
    let end = if bytes.contains(&b'\n') { b'\n' } else { b'\r' };
    bytes
        .split(|&b| b == end)
        .enumerate()
        .flat_map(|(i, raw)| {
            let text = String::from_utf8_lossy(raw);
            let what = if text.contains(ALLOW) { None } else { text.to_lowercase().split('\r').find_map(finding) };
            what.map(|w| (i + 1, w, short(text.trim())))
        })
        .collect()
}

/// What a lowercased line matches, if anything.
fn finding(l: &str) -> Option<&'static str> {
    if !l.contains("adobe") {
        return None;
    }
    if l.contains("adobe systems incorporated")
        || ["copyright", "©", "(c)"].iter().any(|m| l.match_indices(m).any(|(i, _)| names_adobe_next(l.get(i + m.len()..).unwrap_or_default())))
    {
        return Some("Adobe copyright notice");
    }
    let t = l.trim_start();
    let stamp = (t.starts_with("%%creator:") && t.contains("adobe"))
        || (t.starts_with("%%title:") && t.contains("adobe illustrator"))
        || ["/creator", "/producer"]
            .iter()
            .any(|k| l.match_indices(k).any(|(i, _)| l.get(i + k.len()..).unwrap_or_default().trim_start().starts_with("(adobe")))
        || l.match_indices("xmp:creatortool").any(|(i, _)| {
            l.get(i + "xmp:creatortool".len()..).unwrap_or_default().trim_start_matches(['=', '"', '\'', '>', ' ']).starts_with("adobe")
        });
    stamp.then_some("made with an Adobe application")
}

/// Does `rest` (after a copyright mark) name Adobe once years, marks and punctuation are skipped?
fn names_adobe_next(mut rest: &str) -> bool {
    loop {
        let r = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c.is_whitespace() || ",.:-–—".contains(c));
        let r = ["(c)", "©", "(r)", "copyright"].iter().fold(r, |r, m| r.strip_prefix(m).unwrap_or(r));
        if r.len() == rest.len() {
            return r.starts_with("adobe");
        }
        rest = r;
    }
}

fn short(s: &str) -> String {
    match s.char_indices().nth(120) {
        Some((i, _)) => format!("{}…", s.get(..i).unwrap_or(s)),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<(usize, &'static str)> {
        scan_bytes(text.as_bytes()).into_iter().map(|(l, w, _)| (l, w)).collect()
    }

    #[test]
    fn copyright_notices() {
        for notice in [
            "Copyright 2010-2024 Adobe (http://www.adobe.com/)",
            "% Copyright (c) 1997 Adobe Systems Incorporated. All Rights Reserved.",
            "© 2023 Adobe. All rights reserved.",
            "(C) Adobe Inc.",
            "Copyright © 1984-2019, Adobe",
            "%%Copyright: Adobe Systems Incorporated",
        ] {
            assert_eq!(found(notice), [(1, "Adobe copyright notice")], "{notice}");
        }
        for fine in [
            "fails on Adobe copyright notices and on fixtures made with Adobe applications",
            "Copyright 2026 the VectorCraft contributors; compare with Adobe's docs",
            "%!PS-Adobe-3.0 EPSF-3.0",
            "Adobe Glyph List names",
        ] {
            assert!(found(fine).is_empty(), "{fine}");
        }
    }

    #[test]
    fn creator_stamps() {
        let stamped = [
            "%%Creator: Adobe Illustrator(R) 24.0",
            "%%Creator:Adobe Photoshop",
            "%%Title: (Adobe Illustrator file)",
            "<< /Creator (Adobe Illustrator 27.0) /Producer (Adobe PDF library 17.00) >>",
            "/Producer(Adobe PDF Library)",
            "<xmp:CreatorTool>Adobe Illustrator 27.0 (Macintosh)</xmp:CreatorTool>",
            "xmp:CreatorTool=\"Adobe InDesign 18.0\"",
        ];
        for s in stamped {
            assert_eq!(found(s), [(1, "made with an Adobe application")], "{s}");
        }
        for fine in ["%%Creator: VectorCraft", "/Creator (VectorCraft) % reads Adobe-style DSC", "xmp:CreatorTool=\"VectorCraft\" for Adobe readers"]
        {
            assert!(found(fine).is_empty(), "{fine}");
        }
    }

    #[test]
    fn lines_binary_and_markers() {
        assert_eq!(found("one\r\ntwo\r\n%%Creator: Adobe Illustrator\r\n"), [(3, "made with an Adobe application")]);
        assert_eq!(found("a\rb\r/Producer (Adobe PDF)\r"), [(3, "made with an Adobe application")]);
        let mut binary = vec![0u8, 0xff, 0x80, b'\n'];
        binary.extend_from_slice(b"4 0 obj << /Producer (Adobe PDF Library 15.0) >>\n\x00\xfe");
        assert_eq!(found(&String::from_utf8_lossy(&binary)), [(2, "made with an Adobe application")]);
        assert_eq!(scan_bytes(&binary).len(), 1, "raw bytes, not just text");
        assert!(found("const AI: &str = \"%%Creator: Adobe Illustrator(R) 8.0\"; // cleanroom-ok\n").is_empty());
    }

    #[test]
    fn injected_files_fail_the_gate() {
        let dir = std::env::temp_dir().join(format!("vc-cleanroom-{}", std::process::id()));
        let file = |p: &str, bytes: &[u8]| {
            let path = dir.join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        file("tests/fixtures/x.ai", b"%!PS-Adobe-3.0\n%%Creator: Adobe Illustrator(R) 24.0\n");
        file("docs/x.md", b"Written from the spec.\n");
        file("NOTICE", b"Copyright 1990-2023 Adobe. All rights reserved.\n");
        let files: Vec<String> = ["tests/fixtures/x.ai", "docs/x.md", "NOTICE", "missing.pdf"].map(String::from).to_vec();
        let hits = scan(&dir, &files);
        let _ = std::fs::remove_dir_all(&dir);
        let found: Vec<(&str, usize)> = hits.iter().map(|h| (h.path.as_str(), h.line)).collect();
        assert_eq!(found, [("tests/fixtures/x.ai", 2)]);
    }

    #[test]
    fn the_tree_is_clean() {
        let root = crate::root();
        // Outside a git checkout there is no file list to check.
        let Ok(files) = crate::repo_files(&root) else { return };
        let hits = scan(&root, &files);
        assert!(hits.is_empty(), "{hits:#?}");
    }
}

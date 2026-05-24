use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{ParsedDesktopFile, RawFile};

fn is_desktop(p: &PathBuf) -> bool {
    p.is_file() && p.extension().map(|s| s == "desktop").unwrap_or(false)
}

fn build_file(path: PathBuf) -> Option<RawFile> {
    if !is_desktop(&path) {
        return None;
    }

    let name = path.file_stem()?.to_string_lossy().into_owned();
    Some(RawFile { name, path })
}

pub fn load_files() -> std::io::Result<Vec<RawFile>> {
    let mut out: Vec<RawFile> = Vec::new();

    for entry in fs::read_dir("/usr/share/applications/")? {
        let entry = entry?;
        if let Some(file) = build_file(entry.path()) {
            out.push(file);
        }
    }
    Ok(out)
}

pub fn parse_file(file_p: &Path) -> Result<ParsedDesktopFile, std::io::Error> {
    let mut hash: HashMap<String, String> = HashMap::new();

    let content = File::open(file_p)?;
    let reader = BufReader::new(content);

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with('[') || line.starts_with('#') || line.is_empty() {
            continue;
        }

        let Some((k, v)) = line.split_once('=') else {
            continue;
        };

        hash.insert(k.trim().to_string(), v.trim().to_string());
    }

    let name = {
        let cands = locale_candidates_from_env();
        // try Name[full_tag], then Name[lang], then Name, then Unknown
        cands
            .iter()
            .find_map(|tag| hash.get(&format!("Name[{}]", tag)).cloned())
            .or_else(|| hash.get("Name").cloned())
            .unwrap_or_else(|| "Unknown".to_string())
    };

    Ok(ParsedDesktopFile {
        name,
        exec: hash.remove("Exec").unwrap_or("Unknown".to_string()),
        icon: hash.remove("Icon").unwrap_or("Unknown".to_string()),
    })
}

fn locale_candidates_from_env() -> Vec<String> {
    // prefer LC_MESSAGES, then LANG; strip encoding and produce fallbacks
    let raw = std::env::var("LC_MESSAGES")
        .ok()
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();
    let root = raw
        .split('.')
        .next()
        .unwrap_or("")
        .split('@')
        .next()
        .unwrap_or("")
        .to_string();
    if root.is_empty() {
        return Vec::new();
    }

    let mut cands = Vec::new();
    cands.push(root.clone()); // full tag, e.g. "pt_BR"
    let lang_only = root.split('_').next().unwrap_or("").to_string();
    if !lang_only.is_empty() && lang_only != root {
        cands.push(lang_only);
    }
    cands
}
// #[cfg(test)]
// mod test {
//     use super::*;

//     // #[test]
//     // fn test_parse_file() {
//     //     parse_file();
//     // }
// }

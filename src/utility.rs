use std::{
    collections::HashMap,
    env,
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

    Ok(ParsedDesktopFile {
        name: hash.remove("Name").unwrap_or("Unknown".to_string()),
        exec: hash.remove("Exec").unwrap_or("Unknown".to_string()),
        icon: hash.remove("Icon").unwrap_or("Unknown".to_string()),
    })
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     // #[test]
//     // fn test_parse_file() {
//     //     parse_file();
//     // }
// }

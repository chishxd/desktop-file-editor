use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc,
};

use crate::{DbUpdateResult, ParsedDesktopFile, RawFile};

fn is_desktop(p: &Path) -> bool {
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
    let mut map: HashMap<String, RawFile> = HashMap::new();

    for entry in fs::read_dir("/usr/share/applications/")? {
        let entry = entry?;
        if let Some(file) = build_file(entry.path()) {
            let key = file
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            map.insert(key, file);
        }
    }
    let local_dir = PathBuf::from(std::env::var("HOME").unwrap()).join(".local/share/applications");
    if local_dir.is_dir() {
        for entry in fs::read_dir(local_dir)? {
            let entry = entry?;
            if let Some(file) = build_file(entry.path()) {
                let key = file
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                map.insert(key, file);
            }
        }
    }
    let mut files: Vec<RawFile> = map.into_values().collect();
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(files)
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

pub fn save_desktop_file(
    path: &Path,
    new_values: &ParsedDesktopFile,
) -> Result<(), std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    for line in &mut lines {
        if line.starts_with("Name=") {
            *line = format!("Name={}", new_values.name);
        } else if line.starts_with("Exec") {
            *line = format!("Exec={}", new_values.exec);
        } else if line.starts_with("Icon") {
            *line = format!("Icon={}", new_values.icon);
        }
    }

    let home = std::env::var("HOME").expect("HOME not set");
    let dest_dir = std::path::PathBuf::from(format!("{}/.local/share/applications", home));
    std::fs::create_dir_all(&dest_dir)?;
    let filename = path.file_name().unwrap(); // e.g., "firefox.desktop"
    let dest_path = dest_dir.join(filename);

    let content = lines.join("\n") + "\n";
    std::fs::write(dest_path, content)?;
    Ok(())
}

pub fn spawn_update_desktop_database() -> mpsc::Receiver<DbUpdateResult> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = std::process::Command::new("update-desktop-database")
            .arg(format!(
                "{}/.local/share/applications",
                std::env::var("HOME").unwrap()
            ))
            .status();

        let message = match result {
            Ok(status) if status.success() => DbUpdateResult::Updated,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => DbUpdateResult::MissingBinary,
            Err(err) => DbUpdateResult::Failed(err.to_string()),
            Ok(status) => DbUpdateResult::Failed(format!("exit status: {status}")),
        };

        let _ = tx.send(message);
    });

    rx
}

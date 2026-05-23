use std::{fs, path::PathBuf};

use crate::File;

fn is_desktop(p: &PathBuf) -> bool {
    p.is_file() && p.extension().map(|s| s == "desktop").unwrap_or(false)
}

fn build_file(path: PathBuf) -> Option<File> {
    if !is_desktop(&path) {
        return None;
    }

    let name = path.file_stem()?.to_string_lossy().into_owned();
    Some(File { name, path })
}

pub fn load_files() -> std::io::Result<Vec<File>> {
    let mut out: Vec<File> = Vec::new();

    for entry in fs::read_dir("/usr/share/applications/")? {
        let entry = entry?;
        if let Some(file) = build_file(entry.path()) {
            out.push(file);
        }
    }
    Ok(out)
}

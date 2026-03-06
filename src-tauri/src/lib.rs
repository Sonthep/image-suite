use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::Emitter;
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "avif"];

// ─── Shared structs ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ScanResult {
    pub total: usize,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub dir: String,
    pub filename: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RenameEvent {
    pub kind: String,
    pub message: String,
    pub done: usize,
    pub skipped: usize,
    pub total: usize,
}

// ─── Stats structs ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct FolderStats {
    pub total_images: usize,
    pub total_size_kb: u64,
    pub by_extension: HashMap<String, usize>,
    pub subfolders: usize,
}

// ─── Duplicate structs ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct DuplicateGroup {
    pub name: String,
    pub files: Vec<String>, // full paths
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DuplicateResult {
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicates: usize,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{}", ext)))
}

fn collect_images(path: &str) -> Vec<(String, String)> {
    // returns (dir, filename)
    let mut out = Vec::new();
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if is_image(&fname) {
                let dir = entry
                    .path()
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_string_lossy()
                    .to_string();
                out.push((dir, fname));
            }
        }
    }
    out
}

// ─── Command: scan_folder ─────────────────────────────────────────────────────

#[tauri::command]
fn scan_folder(path: String) -> Result<ScanResult, String> {
    let pairs = collect_images(&path);
    let total = pairs.len();
    let files = pairs
        .into_iter()
        .map(|(dir, filename)| FileEntry { dir, filename })
        .collect();
    Ok(ScanResult { total, files })
}

// ─── Command: rename_files ────────────────────────────────────────────────────

#[tauri::command]
async fn rename_files(app: tauri::AppHandle, files: Vec<FileEntry>) -> Result<String, String> {
    let total = files.len();
    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut current_folder = String::new();
    let mut count = 1usize;

    for entry in &files {
        let folder_name = Path::new(&entry.dir)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if folder_name != current_folder {
            count = 1;
            current_folder = folder_name.clone();
        }

        let old_path = Path::new(&entry.dir).join(&entry.filename);
        let ext = Path::new(&entry.filename)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let new_filename = format!("{}_{}.{}", folder_name, count, ext);
        let new_path = Path::new(&entry.dir).join(&new_filename);

        let (msg, kind) = if !new_path.exists() {
            match std::fs::rename(&old_path, &new_path) {
                Ok(_) => {
                    done += 1;
                    (format!("✅  {}  →  {}", entry.filename, new_filename), "ok".to_string())
                }
                Err(e) => {
                    skipped += 1;
                    (format!("❌  {} ({})", entry.filename, e), "err".to_string())
                }
            }
        } else {
            skipped += 1;
            (format!("⚠️  ข้าม: {} (มีอยู่แล้ว)", new_filename), "skip".to_string())
        };

        count += 1;
        let _ = app.emit("rename_progress", RenameEvent { kind, message: msg, done, skipped, total });
    }

    Ok(format!("เสร็จสิ้น! สำเร็จ {} ไฟล์ | ข้าม {} ไฟล์", done, skipped))
}

// ─── Command: get_folder_stats ────────────────────────────────────────────────

#[tauri::command]
fn get_folder_stats(path: String) -> Result<FolderStats, String> {
    let mut total_images = 0usize;
    let mut total_size_kb = 0u64;
    let mut by_extension: HashMap<String, usize> = HashMap::new();
    let mut subfolders = 0usize;

    for entry in WalkDir::new(&path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.depth() == 0 { continue; }
        if entry.file_type().is_dir() {
            subfolders += 1;
        } else if entry.file_type().is_file() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if is_image(&fname) {
                total_images += 1;
                if let Ok(meta) = entry.metadata() {
                    total_size_kb += meta.len() / 1024;
                }
                let ext = Path::new(&fname)
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                *by_extension.entry(ext).or_insert(0) += 1;
            }
        }
    }

    Ok(FolderStats { total_images, total_size_kb, by_extension, subfolders })
}

// ─── Command: find_duplicates ─────────────────────────────────────────────────

#[tauri::command]
fn find_duplicates(path: String) -> Result<DuplicateResult, String> {
    // Group by (filename stem + size) as a lightweight duplicate heuristic
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for entry in WalkDir::new(&path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if is_image(&fname) {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let key = format!("{}__{}", fname.to_lowercase(), size);
                map.entry(key)
                    .or_default()
                    .push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    let mut groups: Vec<DuplicateGroup> = map
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, files)| {
            let name = k.split("__").next().unwrap_or("").to_string();
            DuplicateGroup { name, files }
        })
        .collect();

    groups.sort_by(|a, b| a.name.cmp(&b.name));
    let total_duplicates = groups.iter().map(|g| g.files.len() - 1).sum();

    Ok(DuplicateResult { groups, total_duplicates })
}

// ─── Command: list_images_in_folder ──────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageInfo {
    pub path: String,
    pub filename: String,
    pub ext: String,
    pub size_kb: u64,
}

#[tauri::command]
fn list_images_in_folder(path: String) -> Result<Vec<ImageInfo>, String> {
    let mut out = Vec::new();
    for entry in WalkDir::new(&path)
        .follow_links(false)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if is_image(&fname) {
                let ext = Path::new(&fname)
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                let size_kb = entry.metadata().map(|m| m.len() / 1024).unwrap_or(0);
                out.push(ImageInfo {
                    path: entry.path().to_string_lossy().to_string(),
                    filename: fname,
                    ext,
                    size_kb,
                });
            }
        }
    }
    Ok(out)
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            rename_files,
            get_folder_stats,
            find_duplicates,
            list_images_in_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

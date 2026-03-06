use anyhow::{Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView, RgbaImage};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

fn list_images(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(OsStr::to_str) {
                let ext = ext.to_ascii_lowercase();
                if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
                    files.push(entry.into_path());
                }
            }
        }
    }
    files
}

fn run_remove_bg_py(input: &Path, out: &Path) -> Result<()> {
    // Call the helper Python script with input and output paths
    let script = Path::new("tools/remove_bg.py");
    let status = Command::new("python")
        .arg(script)
        .arg(input)
        .arg(out)
        .status()
        .with_context(|| format!("failed to spawn python for {}", input.display()))?;
    if !status.success() {
        anyhow::bail!("remove_bg.py returned non-zero for {}", input.display());
    }
    Ok(())
}

fn place_on_canvas(img: DynamicImage, canvas_size: u32, product_size: u32) -> DynamicImage {
    // Resize to fit product_size on longest side
    let mut img = img;
    let (w, h) = img.dimensions();
    let scale = if w >= h { product_size as f64 / w as f64 } else { product_size as f64 / h as f64 };
    let new_w = ((w as f64) * scale).round() as u32;
    let new_h = ((h as f64) * scale).round() as u32;
    let resized = img.resize(new_w, new_h, FilterType::Lanczos3);

    // Create white canvas and paste centered
    let mut canvas = DynamicImage::new_rgb8(canvas_size, canvas_size).to_rgba8();
    let px = (canvas_size - resized.width()) / 2;
    let py = (canvas_size - resized.height()) / 2;
    // If resized has alpha, blend using alpha channel
    let resized_rgba = resized.to_rgba8();
    for y in 0..resized_rgba.height() {
        for x in 0..resized_rgba.width() {
            let p = resized_rgba.get_pixel(x, y);
            let alpha = p[3] as f32 / 255.0;
            let cx = px + x;
            let cy = py + y;
            let base = canvas.get_pixel(cx, cy);
            let blended = [
                ((p[0] as f32) * alpha + (base[0] as f32) * (1.0 - alpha)) as u8,
                ((p[1] as f32) * alpha + (base[1] as f32) * (1.0 - alpha)) as u8,
                ((p[2] as f32) * alpha + (base[2] as f32) * (1.0 - alpha)) as u8,
                255u8,
            ];
            canvas.put_pixel(cx, cy, image::Rgba(blended));
        }
    }
    DynamicImage::ImageRgba8(canvas)
}

fn save_jpeg(img: &DynamicImage, out: &Path, quality: u8) -> Result<()> {
    let mut fout = std::fs::File::create(out).with_context(|| format!("create output {}", out.display()))?;
    let jpg = img.to_rgb8();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut fout, quality);
    enc.encode_image(&DynamicImage::ImageRgb8(jpg))?;
    Ok(())
}

fn process_one(input: &Path, output: &Path, canvas_size: u32, product_size: u32, quality: u8) -> Result<()> {
    // call python helper to produce a temp PNG with alpha
    let mut tmp = NamedTempFile::new().context("create tmp file")?;
    let tmp_path = tmp.path().with_extension("png");
    // ensure path ends with .png
    let tmp_path_str = tmp_path.to_string_lossy().to_string();
    // we will write to that path
    run_remove_bg_py(input, Path::new(&tmp_path_str))?;

    let img = image::open(&tmp_path_str).with_context(|| format!("open tmp png {}", tmp_path_str))?;
    let result = place_on_canvas(img, canvas_size, product_size);
    // ensure output dir exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    save_jpeg(&result, output, quality)?;
    Ok(())
}

fn main() -> Result<()> {
    // simple CLI: pipeline <src_folder> <dst_folder> <canvas> <product> <quality>
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!("Usage: pipeline <src> <dst> <canvas_px> <product_px> <jpg_quality>");
        std::process::exit(2);
    }
    let src = Path::new(&args[1]);
    let dst = Path::new(&args[2]);
    let canvas: u32 = args[3].parse()?;
    let product: u32 = args[4].parse()?;
    let quality: u8 = args[5].parse()?;

    let files = list_images(src);
    if files.is_empty() {
        println!("no files found");
        return Ok(());
    }
    let total = files.len();
    for (i, p) in files.into_iter().enumerate() {
        let base = p.file_stem().and_then(OsStr::to_str).unwrap_or("file");
        let out_path = dst.join(format!("{}.jpg", base));
        match process_one(&p, &out_path, canvas, product, quality) {
            Ok(()) => println!("[{}/{}] ✅ {}", i + 1, total, out_path.display()),
            Err(e) => eprintln!("[{}/{}] ❌ {} -> {}", i + 1, total, p.display(), e),
        }
    }
    Ok(())
}

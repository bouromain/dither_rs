use anyhow::{Context, Result};
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Luma};
use log::{error, info, warn};
use rayon::prelude::*;
use std::env;
use std::fs::{self};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_MAX_IMAGE_SIDE: u32 = 800;
const DEFAULT_BAYER_ORDER: usize = 8;

/// Configuration for the image processing
#[derive(Debug)]
struct Config {
    dir_path: PathBuf,
    max_image_side: u32,
    bayer_order: usize,
}

impl Config {
    fn from_args() -> Result<Self> {
        let args: Vec<String> = env::args().collect();
        if args.len() < 2 {
            anyhow::bail!(
                "Usage: {} <path-to-images> [max_image_side] [bayer_order]",
                args.get(0).unwrap_or(&String::from("program"))
            );
        }

        let dir_path = PathBuf::from(&args[1]);
        if !dir_path.exists() {
            anyhow::bail!("Directory does not exist: {}", dir_path.display());
        }

        let max_image_side = args
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_IMAGE_SIDE);

        let bayer_order = args
            .get(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_BAYER_ORDER);

        if !bayer_order.is_power_of_two() {
            anyhow::bail!("Bayer order must be a power of 2");
        }

        Ok(Config {
            dir_path,
            max_image_side,
            bayer_order,
        })
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::from_args()?;

    info!(
        "Starting image processing in directory: {}",
        config.dir_path.display()
    );

    let files = list_image_files(&config.dir_path)?;
    if files.is_empty() {
        warn!("No image files found in the specified directory");
        return Ok(());
    }

    // Process images in parallel
    files.par_iter().for_each(|file| {
        match process_image(file, config.max_image_side, config.bayer_order) {
            Ok(_) => info!("✅ Successfully processed: {}", file.display()),
            Err(e) => error!("❌ Failed to process {}: {}", file.display(), e),
        }
    });

    Ok(())
}

fn list_image_files(dir_path: &Path) -> Result<Vec<PathBuf>> {
    const ALLOWED_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "webp", "tiff", "bmp"];

    let files: Vec<PathBuf> = WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();

    Ok(files)
}

fn process_image(file: &Path, max_image_side: u32, bayer_order: usize) -> Result<()> {
    let img =
        image::open(file).with_context(|| format!("Failed to open image: {}", file.display()))?;

    let dithered_img = apply_bayer_dithering_and_resize(img, bayer_order, max_image_side);
    save_image(&dithered_img, file)?;

    Ok(())
}

fn apply_bayer_dithering_and_resize(
    img: DynamicImage,
    order: usize,
    max_image_side: u32,
) -> DynamicImage {
    // Resize logic
    let (width, height) = img.dimensions();
    let max_side = width.max(height);
    let scale = if max_side > max_image_side {
        max_image_side as f64 / max_side as f64
    } else {
        1.0
    };

    let new_width = (width as f64 * scale).round() as u32;
    let new_height = (height as f64 * scale).round() as u32;
    let resized_img = imageops::resize(&img, new_width, new_height, imageops::FilterType::Lanczos3);

    // Convert to grayscale and apply dithering
    let bayer_matrix = generate_bayer_matrix(order);
    let max_value = (order * order) as u32;
    let scale_factor = 256 / max_value;

    let mut buffer: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(new_width, new_height);

    for y in 0..new_height {
        for x in 0..new_width {
            let pixel = resized_img.get_pixel(x, y);
            let gray = ((pixel[0] as f32 * 0.299)
                + (pixel[1] as f32 * 0.587)
                + (pixel[2] as f32 * 0.114)) as u32;
            let threshold = bayer_matrix[y as usize % order][x as usize % order] * scale_factor;
            let new_pixel = if gray > threshold { 255 } else { 0 };
            buffer.put_pixel(x, y, Luma([new_pixel as u8]));
        }
    }

    DynamicImage::ImageLuma8(buffer)
}

fn generate_bayer_matrix(order: usize) -> Vec<Vec<u32>> {
    debug_assert!(order.is_power_of_two(), "Bayer order must be a power of 2");

    let mut matrix = vec![vec![0; order]; order];
    let mut size = 1;
    matrix[0][0] = 0;

    while size < order {
        let new_size = size * 2;
        for i in 0..size {
            for j in 0..size {
                let val = matrix[i][j];
                matrix[i][j] = 4 * val + 1;
                matrix[i + size][j] = 4 * val + 2;
                matrix[i][j + size] = 4 * val + 3;
                matrix[i + size][j + size] = 4 * val;
            }
        }
        size = new_size;
    }

    // Normalize the matrix
    for row in matrix.iter_mut() {
        for val in row.iter_mut() {
            *val += 1;
        }
    }

    matrix
}

fn save_image(img: &DynamicImage, original_path: &Path) -> Result<()> {
    let dither_dir = original_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("dithers");

    fs::create_dir_all(&dither_dir)
        .with_context(|| format!("Failed to create directory: {}", dither_dir.display()))?;

    let new_path = dither_dir.join(
        original_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?,
    );

    img.save_with_format(&new_path, ImageFormat::Png)
        .with_context(|| format!("Failed to save image: {}", new_path.display()))?;

    info!("🖼 Image saved to {}", new_path.display());
    Ok(())
}

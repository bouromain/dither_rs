use image::{imageops, DynamicImage, ImageBuffer, ImageFormat, Luma};
use std::env;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <path-to-images> [max_image_side]", args[0]);
        return;
    }
    let dir_path = &args[1];
    let max_image_side = args
        .get(2)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(800);

    let files = list_image_files(dir_path);
    for file in files {
        println!("Processing: {}", file.display());
        if let Ok(img) = image::open(&file) {
            let dithered_img = apply_bayer_dithering_and_resize(img, 8, max_image_side);
            save_image(dithered_img, &file);
        } else {
            println!("❌ Failed to open image: {}", file.display());
        }
    }
}

fn list_image_files(dir_path: &str) -> Vec<PathBuf> {
    let allowed_extensions = vec!["jpg", "jpeg", "png", "gif", "webp", "tiff", "bmp"];

    WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(|ext| allowed_extensions.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect()
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
    let new_width = (width as f64 * scale) as u32;
    let new_height = (height as f64 * scale) as u32;
    let resized_img = imageops::resize(&img, new_width, new_height, imageops::FilterType::Lanczos3);

    // Bayer dithering logic
    let bayer_matrix = generate_bayer_matrix(order);
    let max_value = (order * order) as u32;
    let scale_factor = 256 / max_value;
    let mut buffer: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(new_width, new_height);

    for (x, y, pixel) in resized_img.pixels() {
        let gray = pixel.0[0] as u32;
        let threshold = bayer_matrix[(y as usize % order)][(x as usize % order)] * scale_factor;
        let new_pixel = if gray > threshold { 255 } else { 0 };
        buffer.put_pixel(x, y, Luma([new_pixel as u8]));
    }

    DynamicImage::ImageLuma8(buffer)
}

fn generate_bayer_matrix(order: usize) -> Vec<Vec<u32>> {
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

    // Normalize the matrix by adding 1 to each element
    for i in 0..order {
        for j in 0..order {
            matrix[i][j] += 1;
        }
    }

    matrix
}

fn save_image(img: DynamicImage, original_path: &Path) {
    let dither_dir = original_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("dithers");
    if !dither_dir.exists() {
        fs::create_dir_all(&dither_dir).unwrap();
    }
    let new_path = dither_dir.join(original_path.file_name().unwrap());
    img.save_with_format(&new_path, ImageFormat::Png).unwrap(); // Save as PNG
    println!("🖼 Image saved to {}", new_path.display());
}

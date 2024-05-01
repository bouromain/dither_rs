# Dither_rs

This package is a simple cli application to get more familiar with and learn rust.

It will replicate the function of the following code: 
https://github.com/lowtechmag/solar_v2/blob/main/utils/dither_images.py

In brief, it will:
- list all the image file in a directory recursively 
- downsample them such that the largest dimension is 800px
- apply [Bayer dithering](https://en.wikipedia.org/wiki/Dither) on the image

## Prerequisites
[Ensure you have Rust and Cargo installed on your machine.](https://rustup.rs/)

## Installation

```
# Clone the repository
git clone https://github.com/yourusername/dither_rs.git

# Navigate to the project directory
cd dither_rs

# Build the project
cargo build --release

# Run the program
cargo run -- /path/to/your/images/directory [max_image_side]
```

[max_image_side] is an optional argument that sets the maximum size of the image's largest dimension. If not specified, it defaults to 800 pixels.

# Examples

```
# Process images in a folder with the default settings
cargo run -- /path/to/images

# Process images in a folder, setting the maximum dimension to 1024 pixels
cargo run -- /path/to/images 1024
```

use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Pixel, Rgba, RgbaImage};
use rayon::prelude::*;
use std::fs::create_dir_all;
use std::{fmt::Display, path::PathBuf};

mod math;
use anyhow::{Ok, Result};
use math::{Interpolation, SphericalAngle, Vector3};

use crate::math::{reinhard_tone_mapping_rgb, reinhard_tone_mapping_rgba};

fn main() -> Result<()> {
    let config: Config = argh::from_env();
    let path = &config.input;
    let start_time = std::time::Instant::now();

    let mut reader = image::ImageReader::open(path)?;
    if config.unlimited {
        reader.no_limits();
    }
    let img = reader.decode()?;
    let elapsed = start_time.elapsed();
    println!("Read and Parse: {elapsed:?}");
    let width = img.width();
    let height = img.height();
    if width != height * 2 {
        panic!("The image width must be exactly twice the height.")
    }

    create_dir_all(&config.output)?;
    let start_time = std::time::Instant::now();

    // convert equirect to cubemaps
    let mut images = reproject(&config, img);
    let elapsed = start_time.elapsed();
    println!("Convert: {:?}", elapsed);
    if config.rotate {
        let start_time = std::time::Instant::now();
        images = rotate(images);
        let elapsed = start_time.elapsed();
        println!("Rotate: {:?}", elapsed);
    }
    let start_time = std::time::Instant::now();
    let size = config.size;

    use image::EncodableLayout as _;

    // tonemapping, format conversion + writting to disk
    images.into_iter().for_each(|(img, side)| {
        let exposure = config.exposure;
        // tonemapped image will be converted to rgb8 or rgba8
        let img = if config.tone_mapping {
            match img {
                DynamicImage::ImageRgb32F(image_buffer) => {
                    let (width, height) = image_buffer.dimensions();
                    let mut new_image = DynamicImage::new_rgb8(width, height);
                    for x in 0..width {
                        for y in 0..height {
                            let pixel = image_buffer.get_pixel(x, y);
                            let mapped = reinhard_tone_mapping_rgb(*pixel, exposure);
                            new_image.put_pixel(x, y, mapped);
                        }
                    }
                    new_image
                }
                DynamicImage::ImageRgba32F(image_buffer) => {
                    let (width, height) = image_buffer.dimensions();
                    let mut new_image = DynamicImage::new_rgba8(width, height);
                    for x in 0..width {
                        for y in 0..height {
                            let pixel = image_buffer.get_pixel(x, y);
                            let mapped = reinhard_tone_mapping_rgba(*pixel, exposure);
                            new_image.put_pixel(x, y, mapped);
                        }
                    }
                    new_image
                }
                _ => img,
            }
        } else {
            img
        };
        let has_alpha = img.color().has_alpha();
        let buffer = match config.format {
            OutputFormat::Jpg => img.into_rgb8().into_vec(),
            OutputFormat::Png => img.into_rgba8().into_vec(),
            OutputFormat::Webp => {
                if has_alpha {
                    img.into_rgba8().into_vec()
                } else {
                    img.into_rgb8().into_vec()
                }
            }
            OutputFormat::Hdr => {
                let vec = img.into_rgb32f().into_vec();
                bytemuck::cast_slice(&vec).to_vec()
            }
            OutputFormat::Exr => {
                if has_alpha {
                    let vec = img.to_rgba32f().into_vec();
                    bytemuck::cast_slice(&vec).to_vec()
                } else {
                    let vec = img.to_rgb32f().into_vec();
                    bytemuck::cast_slice(&vec).to_vec()
                }
            }
        };
        let color_type = match config.format {
            OutputFormat::Jpg => image::ColorType::Rgb8,
            OutputFormat::Png | OutputFormat::Webp => image::ColorType::Rgba8,
            OutputFormat::Hdr => image::ColorType::Rgb32F,
            OutputFormat::Exr => image::ColorType::Rgba32F,
        };
        image::save_buffer_with_format(
            config.output.join(format!("{}.{}", side, &config.format)),
            &buffer,
            size,
            size,
            color_type,
            config.format.into(),
        )
        .unwrap();
    });
    println!(
        r#"Generated images has been saved in "{}""#,
        config.output.display()
    );

    Ok(())
}
use argh::FromArgs;
/// Configuration of the conversion.
#[derive(FromArgs, Debug, Clone)]
struct Config {
    /// the format of the output images
    #[argh(option, short = 'f', default = "OutputFormat::Png")]
    format: OutputFormat,
    /// interpolation used when sampling source image
    #[argh(option, short = 'i', default = "Interpolation::Linear")]
    interpolation: Interpolation,
    /// the input equirectangular image's path
    #[argh(positional)]
    input: PathBuf,
    /// the directory to put the output images in, creates if not exists
    #[argh(positional)]
    output: PathBuf,
    #[argh(option, short = 's', default = "512")]
    /// size (px) of the output images, width = height
    size: u32,
    /// rotate to a z-up skybox if you use it in a y-up renderer
    #[argh(switch, short = 'r')]
    rotate: bool,
    /// enable tone mapping (Reinhard)
    #[argh(switch, short = 't')]
    tone_mapping: bool,
    /// exposure of tone mapping
    #[argh(option, short = 'e', default = "1.0")]
    exposure: f32,
    /// remove the limits on image size and memory usage (could cause OOM on large images or decompression bombs)
    #[argh(switch, short = 'u')]
    unlimited: bool,
}
#[derive(argh::FromArgValue, Clone, Debug, Copy)]
enum OutputFormat {
    Jpg,
    Png,
    Webp,
    Hdr,
    Exr,
}
impl From<OutputFormat> for image::ImageFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Jpg => image::ImageFormat::Jpeg,
            OutputFormat::Png => image::ImageFormat::Png,
            OutputFormat::Webp => image::ImageFormat::WebP,
            OutputFormat::Hdr => image::ImageFormat::Hdr,
            OutputFormat::Exr => image::ImageFormat::OpenExr,
        }
    }
}
impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Jpg => write!(f, "jpg"),
            OutputFormat::Png => write!(f, "png"),
            OutputFormat::Webp => write!(f, "webp"),
            OutputFormat::Hdr => write!(f, "hdr"),
            OutputFormat::Exr => write!(f, "exr"),
        }
    }
}
impl OutputFormat {
    pub fn is_rgb(&self) -> bool {
        matches!(self, OutputFormat::Jpg)
    }
    pub fn is_hdr(&self) -> bool {
        matches!(self, OutputFormat::Hdr | OutputFormat::Exr)
    }
}

#[derive(Clone, Copy)]
pub enum Side {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Front => write!(f, "front"),
            Side::Back => write!(f, "back"),
            Side::Left => write!(f, "left"),
            Side::Right => write!(f, "right"),
            Side::Top => write!(f, "top"),
            Side::Bottom => write!(f, "bottom"),
        }
    }
}

/// convert 1 equirect image to cubemaps (6 squared images)
fn reproject(config: &Config, img: DynamicImage) -> Vec<(DynamicImage, Side)> {
    // use rayon::ParIter;
    use Side::*;
    let size = config.size;
    let interpolation = &config.interpolation;
    [Front, Back, Left, Right, Top, Bottom]
        .par_iter()
        .map(|side| {
            let size_int = size;
            let size = size as f32;
            let mut square = DynamicImage::new(size_int, size_int, img.color());
            for x in 0..size_int {
                let xf = x as f32;
                for y in 0..size_int {
                    let yf = y as f32;
                    // TODO performance gain if i move the match out of the loop?
                    let pos = match side {
                        Front => Vector3::new(0.5, xf / size - 0.5, yf / size - 0.5),
                        Back => Vector3::new(-0.5, 0.5 - xf / size, yf / size - 0.5),
                        Left => Vector3::new(-(xf / size - 0.5), 0.5, yf / size - 0.5),
                        Right => Vector3::new(xf / size - 0.5, -0.5, yf / size - 0.5),
                        Top => Vector3::new(xf / size - 0.5, 0.5 - yf / size, -0.5),
                        Bottom => Vector3::new(xf / size - 0.5, yf / size - 0.5, 0.5),
                    };
                    let spr = SphericalAngle::from_normalized_vector(pos.normalize());
                    let uv = spr.to_uv();
                    if let Some(p) = interpolation.sample(&img, uv) {
                        square.put_pixel(x, y, p);
                    }
                }
            }
            (square, *side)
        })
        .collect()
}
pub fn rotate(entries: Vec<(DynamicImage, Side)>) -> Vec<(DynamicImage, Side)> {
    use image::imageops::*;
    entries
        .into_par_iter()
        .map(|(img, side)| {
            let image = match side {
                Side::Top | Side::Right => img,
                Side::Bottom => DynamicImage::from(rotate180(&img)),
                Side::Left => DynamicImage::from(rotate180(&img)),
                Side::Front => DynamicImage::from(rotate270(&img)),
                Side::Back => DynamicImage::from(rotate90(&img)),
            };
            (image, side)
        })
        .collect()
}

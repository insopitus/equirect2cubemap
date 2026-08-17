use image::{imageops::sample_nearest, DynamicImage, Rgb, Rgba};

/// spherical coord without radius
#[derive(Debug)]
pub struct SphericalAngle {
    /// rotation on pole axis
    pub theta: f32,
    /// angle on equator
    pub phi: f32,
}

impl SphericalAngle {
    pub fn from_normalized_vector(value: Vector3) -> Self {
        let theta = value.y.atan2(value.x);
        let phi = value.z.asin();
        Self { theta, phi }
    }
    pub fn to_uv(&self) -> (f32, f32) {
        use std::f32::consts::PI;

        (self.theta / (2.0 * PI) + 0.5, self.phi / PI + 0.5)
    }
}

#[derive(argh::FromArgValue, Clone, Debug)]
pub enum Interpolation {
    Linear,
    Nearest,
}
impl Interpolation {
    pub fn sample(&self, img: &DynamicImage, uv: (f32, f32)) -> Rgba<u8> {
        use image::imageops::sample_bilinear;
        match self {
            Self::Linear => sample_bilinear(img, uv.0, uv.1),
            Self::Nearest => sample_nearest(img, uv.0, uv.1),
        }
        .unwrap_or(Rgba::<u8>([0, 0, 0, 255]))
    }
}

pub fn reinhard_tone_mapping_rgba(color: Rgba<f32>, exposure: f32) -> Rgba<u8> {
    let r = (color[0] * exposure) / (1.0 + color[0] * exposure);
    let g = (color[1] * exposure) / (1.0 + color[1] * exposure);
    let b = (color[2] * exposure) / (1.0 + color[2] * exposure);
    let r = (r * 255.0).round() as u8;
    let g = (g * 255.0).round() as u8;
    let b = (b * 255.0).round() as u8;
    let a = (color[3] * 255.0).round() as u8;

    [r, g, b, a].into()
}
pub fn reinhard_tone_mapping_rgb(color: Rgb<f32>, exposure: f32) -> Rgba<u8> {
    let r = (color[0] * exposure) / (1.0 + color[0] * exposure);
    let g = (color[1] * exposure) / (1.0 + color[1] * exposure);
    let b = (color[2] * exposure) / (1.0 + color[2] * exposure);
    let r = (r * 255.0).round() as u8;
    let g = (g * 255.0).round() as u8;
    let b = (b * 255.0).round() as u8;

    [r, g, b, 255].into()
}

#[derive(Debug, Copy, Clone)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    /// return a normalized version of this vector
    pub fn normalize(&self) -> Self {
        let len = self.len();
        Self {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }
    /// make this vector3 normalized
    // pub fn normalize_mut(&mut self) {
    //     let len = self.len();
    //     self.x /= len;
    //     self.y /= len;
    //     self.z /= len;
    // }
    pub fn len(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    // pub fn len_squared(&self) -> f32 {
    //     self.x * self.x + self.y * self.y + self.z * self.z
    // }
}

use image::{GenericImageView, Pixel, Primitive};
use num_traits::{NumCast, Zero};

#[inline]
/// wrap on the horizontal axis
fn map_x(x: f32, width: u32) -> u32 {
    let w = width as f32;
    let mut t = x % w;
    if t < 0.0 {
        t += w;
    }
    (t as u32) % width
}

#[inline]
/// clamp on the vertical axis
fn map_y(y: f32, height: u32) -> u32 {
    y.clamp(0.0, (height.saturating_sub(1)) as f32) as u32
}

pub fn sample<P>(img: &impl GenericImageView<Pixel = P>, u: f32, v: f32, method: SampleMethod) -> P
where
    P: Pixel + 'static,
    P::Subpixel: Primitive + NumCast + Zero,
{
    let (w, h) = img.dimensions();
    let x = u * w as f32 - 0.5;
    let y = v * h as f32 - 0.5;

    match method {
        SampleMethod::Nearest => sample_nearest(img, x, y),
        SampleMethod::Linear => sample_bilinear(img, x, y),
        SampleMethod::CatmullRom => sample_kernel(img, x, y, catmull_rom, 2.0),
        SampleMethod::Mitchell => sample_kernel(img, x, y, mitchell_netravali, 2.0),
        SampleMethod::Lanczos2 => sample_kernel(img, x, y, lanczos2, 2.0),
        SampleMethod::Lanczos3 => sample_kernel(img, x, y, lanczos3, 3.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleMethod {
    Nearest,
    Linear,
    CatmullRom,
    Mitchell,
    Lanczos2,
    Lanczos3,
}
impl argh::FromArgValue for SampleMethod {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value {
            "nearest" => Ok(SampleMethod::Nearest),
            "linear" | "bilinear" => Ok(SampleMethod::Linear),
            "catmull-rom" | "catmull_rom" | "catmullrom" => Ok(SampleMethod::CatmullRom),
            "mitchell" => Ok(SampleMethod::Mitchell),
            "lanczos2" => Ok(SampleMethod::Lanczos2),
            "lanczos3" => Ok(SampleMethod::Lanczos3),
            _ => Err("invalid interpolation method".to_string()),
        }
    }
}

fn sample_nearest<P>(img: &impl GenericImageView<Pixel = P>, x: f32, y: f32) -> P
where
    P: Pixel + 'static,
{
    let (w, h) = img.dimensions();
    let px = map_x(x.round(), w);
    let py = map_y(y.round(), h);
    img.get_pixel(px, py)
}

fn sample_bilinear<P>(img: &impl GenericImageView<Pixel = P>, x: f32, y: f32) -> P
where
    P: Pixel + 'static,
    P::Subpixel: Primitive + NumCast + Zero,
{
    let (w, h) = img.dimensions();

    let x0 = x.floor();
    let y0 = y.floor();
    let x1 = x0 + 1.0;
    let y1 = y0 + 1.0;

    let fx = x - x0;
    let fy = y - y0;

    let p00 = img.get_pixel(map_x(x0, w), map_y(y0, h));
    let p10 = img.get_pixel(map_x(x1, w), map_y(y0, h));
    let p01 = img.get_pixel(map_x(x0, w), map_y(y1, h));
    let p11 = img.get_pixel(map_x(x1, w), map_y(y1, h));

    interpolate_bilinear_pixels(p00, p10, p01, p11, fx, fy)
}

fn interpolate_bilinear_pixels<P>(p00: P, p10: P, p01: P, p11: P, fx: f32, fy: f32) -> P
where
    P: Pixel,
    P::Subpixel: Primitive + NumCast + Zero,
{
    let channels = P::CHANNEL_COUNT as usize;

    let mut out = [P::Subpixel::zero(); 4];
    for c in 0..channels {
        let v00: f32 = NumCast::from(p00.channels()[c]).unwrap_or(0.0);
        let v10: f32 = NumCast::from(p10.channels()[c]).unwrap_or(0.0);
        let v01: f32 = NumCast::from(p01.channels()[c]).unwrap_or(0.0);
        let v11: f32 = NumCast::from(p11.channels()[c]).unwrap_or(0.0);

        let top = v00 * (1.0 - fx) + v10 * fx;
        let bottom = v01 * (1.0 - fx) + v11 * fx;
        let value = top * (1.0 - fy) + bottom * fy;

        out[c] = NumCast::from(value.round()).unwrap_or(P::Subpixel::zero());
    }

    *P::from_slice(&out[..channels])
}

fn sample_kernel<P, F>(
    img: &impl GenericImageView<Pixel = P>,
    x: f32,
    y: f32,
    kernel: F,
    support: f32,
) -> P
where
    P: Pixel + 'static,
    P::Subpixel: Primitive + NumCast + Zero,
    F: Fn(f32) -> f32,
{
    let (w, h) = img.dimensions();
    let channels = P::CHANNEL_COUNT as usize;

    let x_start = (x - support).floor() as i32;
    let x_end = (x + support).ceil() as i32;
    let y_start = (y - support).floor() as i32;
    let y_end = (y + support).ceil() as i32;

    let mut sum = [0.0f32; 4];
    let mut weight_sum = 0.0f32;

    for j in y_start..=y_end {
        let wy = kernel(y - j as f32);
        if wy == 0.0 {
            continue;
        }

        for i in x_start..=x_end {
            let wx = kernel(x - i as f32);
            let weight = wx * wy;
            if weight == 0.0 {
                continue;
            }

            let px = map_x(i as f32, w);
            let py = map_y(j as f32, h);
            let pixel = img.get_pixel(px, py);

            for c in 0..channels {
                let v: f32 = NumCast::from(pixel.channels()[c]).unwrap_or(0.0);
                sum[c] += v * weight;
            }
            weight_sum += weight;
        }
    }

    if weight_sum > 0.0 {
        for c in 0..channels {
            sum[c] /= weight_sum;
        }
    }

    let mut out = [P::Subpixel::zero(); 4];
    for c in 0..channels {
        out[c] = to_subpixel(sum[c]);
    }

    *P::from_slice(&out[..channels])
}

#[inline]
fn catmull_rom(t: f32) -> f32 {
    let t = t.abs();
    if t < 1.0 {
        1.5 * t.powi(3) - 2.5 * t.powi(2) + 1.0
    } else if t < 2.0 {
        -0.5 * t.powi(3) + 2.5 * t.powi(2) - 4.0 * t + 2.0
    } else {
        0.0
    }
}

#[inline]
fn mitchell_netravali(t: f32) -> f32 {
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    let t = t.abs();
    if t < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * t.powi(3)
            + (-18.0 + 12.0 * b + 6.0 * c) * t.powi(2)
            + (6.0 - 2.0 * b))
            / 6.0
    } else if t < 2.0 {
        ((-b - 6.0 * c) * t.powi(3)
            + (6.0 * b + 30.0 * c) * t.powi(2)
            + (-12.0 * b - 48.0 * c) * t
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

#[inline]
fn lanczos2(t: f32) -> f32 {
    let t = t.abs();
    if t < 1e-6 {
        1.0
    } else if t < 2.0 {
        let pi_t = std::f32::consts::PI * t;
        (pi_t.sin() / pi_t) * ((pi_t / 2.0).sin() / (pi_t / 2.0))
    } else {
        0.0
    }
}

#[inline]
fn lanczos3(t: f32) -> f32 {
    let t = t.abs();
    if t < 1e-6 {
        1.0
    } else if t < 3.0 {
        let pi_t = std::f32::consts::PI * t;
        (pi_t.sin() / pi_t) * ((pi_t / 3.0).sin() / (pi_t / 3.0))
    } else {
        0.0
    }
}

/// clamp numbers rather than unwrap to 0.0;
#[inline]
fn to_subpixel<S>(value: f32) -> S
where
    S: Primitive + NumCast,
{
    let is_float = NumCast::from(S::DEFAULT_MAX_VALUE)
        .map(|max: f32| max == 1.0)
        .unwrap_or(false);
    let v = if is_float { value } else { value.round() };

    NumCast::from(v).unwrap_or_else(|| {
        if value < 0.0 {
            S::DEFAULT_MIN_VALUE
        } else {
            S::DEFAULT_MAX_VALUE
        }
    })
}

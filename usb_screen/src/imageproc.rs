//! Geometric transformations of images. This includes rotations, translation, and general
//! projective transformations.

use core::ops::Mul;
use micromath::F32Ext;

use crate::rgb565::{Rgb565Image, Rgb565Pixel};

#[derive(Copy, Clone, Debug)]
enum TransformationClass {
    Translation,
    Affine,
    Projection,
}

/// A 2d projective transformation, stored as a row major 3x3 matrix.
///
/// Transformations combine by pre-multiplication, i.e. applying `P * Q` is equivalent to
/// applying `Q` and then applying `P`. For example, the following defines a rotation
/// about the point (320.0, 240.0).
///
/// ```
/// use imageproc::geometric_transformations::*;
/// use std::f32::consts::PI;
///
/// let (cx, cy) = (320.0, 240.0);
///
/// let c_rotation = Projection::translate(cx, cy)
///     * Projection::rotate(PI / 6.0)
///     * Projection::translate(-cx, -cy);
/// ```
///
/// See ./examples/projection.rs for more examples.
#[derive(Copy, Clone, Debug)]
pub struct Projection {
    transform: [f32; 9],
    inverse: [f32; 9],
    class: TransformationClass,
}

impl Projection {
    /// Creates a 2d projective transform from a row-major 3x3 matrix in homogeneous coordinates.
    ///
    /// Returns `None` if the matrix is not invertible.
    pub fn from_matrix(transform: [f32; 9]) -> Option<Projection> {
        let transform = normalize(transform);
        let class = class_from_matrix(transform);
        try_inverse(&transform).map(|inverse| Projection {
            transform,
            inverse,
            class,
        })
    }

    /// Combine the transformation with another one. The resulting transformation is equivalent to
    /// applying this transformation followed by the `other` transformation.
    pub fn and_then(self, other: Projection) -> Projection {
        other * self
    }

    /// A translation by (tx, ty).
    #[rustfmt::skip]
    pub fn translate(tx: f32, ty: f32) -> Projection {
        Projection {
            transform: [
                1.0, 0.0, tx,
                0.0, 1.0, ty,
                0.0, 0.0, 1.0
            ],
            inverse: [
                1.0, 0.0, -tx,
                0.0, 1.0, -ty,
                0.0, 0.0, 1.0
            ],
            class: TransformationClass::Translation,
        }
    }

    /// A clockwise rotation around the top-left corner of the image by theta radians.
    #[rustfmt::skip]
    pub fn rotate(theta: f32) -> Projection {
        let (s, c) = theta.sin_cos();
        Projection {
            transform: [
                  c,  -s, 0.0,
                  s,   c, 0.0,
                0.0, 0.0, 1.0
            ],
            inverse: [
                  c,   s, 0.0,
                 -s,   c, 0.0,
                0.0, 0.0, 1.0
            ],
            class: TransformationClass::Affine,
        }
    }

    /// An anisotropic scaling (sx, sy).
    ///
    /// Note that the `warp` function does not change the size of the input image.
    /// If you want to resize an image then use the `imageops` module in the `image` crate.
    #[rustfmt::skip]
    pub fn scale(sx: f32, sy: f32) -> Projection {
        Projection {
            transform: [
                 sx, 0.0, 0.0,
                0.0,  sy, 0.0,
                0.0, 0.0, 1.0
            ],
            inverse: [
                1.0 / sx, 0.0,      0.0,
                0.0,      1.0 / sy, 0.0,
                0.0,      0.0,      1.0
            ],
            class: TransformationClass::Affine,
        }
    }

    /// Inverts the transformation.
    pub fn invert(self) -> Projection {
        Projection {
            transform: self.inverse,
            inverse: self.transform,
            class: self.class,
        }
    }

    // Helper functions used as optimization in warp.
    #[inline(always)]
    fn map_projective(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        let d = t[6] * x + t[7] * y + t[8];
        (
            (t[0] * x + t[1] * y + t[2]) / d,
            (t[3] * x + t[4] * y + t[5]) / d,
        )
    }

    #[inline(always)]
    fn map_affine(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        ((t[0] * x + t[1] * y + t[2]), (t[3] * x + t[4] * y + t[5]))
    }

    #[inline(always)]
    fn map_translation(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        let tx = t[2];
        let ty = t[5];
        (x + tx, y + ty)
    }
}

impl Mul<Projection> for Projection {
    type Output = Projection;

    fn mul(self, rhs: Projection) -> Projection {
        use TransformationClass as TC;
        let t = mul3x3(self.transform, rhs.transform);
        let i = mul3x3(rhs.inverse, self.inverse);

        let class = match (self.class, rhs.class) {
            (TC::Translation, TC::Translation) => TC::Translation,
            (TC::Translation, TC::Affine) => TC::Affine,
            (TC::Affine, TC::Translation) => TC::Affine,
            (TC::Affine, TC::Affine) => TC::Affine,
            (_, _) => TC::Projection,
        };

        Projection {
            transform: t,
            inverse: i,
            class,
        }
    }
}

impl<'a, 'b> Mul<&'b Projection> for &'a Projection {
    type Output = Projection;

    fn mul(self, rhs: &Projection) -> Projection {
        *self * *rhs
    }
}

impl Mul<(f32, f32)> for Projection {
    type Output = (f32, f32);

    fn mul(self, rhs: (f32, f32)) -> (f32, f32) {
        let (x, y) = rhs;
        match self.class {
            TransformationClass::Translation => self.map_translation(x, y),
            TransformationClass::Affine => self.map_affine(x, y),
            TransformationClass::Projection => self.map_projective(x, y),
        }
    }
}

impl<'a, 'b> Mul<&'b (f32, f32)> for &'a Projection {
    type Output = (f32, f32);

    fn mul(self, rhs: &(f32, f32)) -> (f32, f32) {
        *self * *rhs
    }
}

/// Rotates an image clockwise about its center.
/// The output image has the same dimensions as the input. Output pixels
/// whose pre-image lies outside the input image are set to `default`.
pub fn rotate_about_center<'a>(
    image: &Rgb565Image<'a>,
    theta: f32,
    interpolation: Interpolation,
    output: &mut Rgb565Image,
    default: Rgb565Pixel,
)
{
    let (w, h) = (image.width, image.height);
    rotate(
        image,
        (w as f32 / 2.0, h as f32 / 2.0),
        theta,
        interpolation,
        output,
        default,
    )
}

/// Rotates an image clockwise about the provided center by theta radians.
/// The output image has the same dimensions as the input. Output pixels
/// whose pre-image lies outside the input image are set to `default`.
pub fn rotate<'a>(
    image: &Rgb565Image<'a>,
    center: (f32, f32),
    theta: f32,
    interpolation: Interpolation,
    output: &mut Rgb565Image,
    default: Rgb565Pixel,
)
{
    let (cx, cy) = center;
    let projection =
        Projection::translate(cx, cy) * Projection::rotate(theta) * Projection::translate(-cx, -cy);
    warp(image, &projection, interpolation, default,  output)
}

/// Applies a projective transformation to an image.
///
/// The returned image has the same dimensions as `image`. Output pixels
/// whose pre-image lies outside the input image are set to `default`.
///
/// The provided projection defines a mapping from locations in the input image to their
/// corresponding location in the output image.
pub fn warp<'a>(
    image: &Rgb565Image<'a>,
    projection: &Projection,
    interpolation: Interpolation,
    default: Rgb565Pixel,
    output: &mut Rgb565Image,
)
{
    warp_into(image, projection, interpolation, default, output);
}

/// Applies a projective transformation to an image, writing to a provided output.
///
/// See the [`warp`](fn.warp.html) documentation for more information.
pub fn warp_into<'a>(
    image: &Rgb565Image<'a>,
    projection: &Projection,
    interpolation: Interpolation,
    default: Rgb565Pixel,
    out: &mut Rgb565Image
)
{
    let projection = projection.invert();
    let nn = |x, y| interpolate_nearest(image, x, y, default);
    let wp = |x, y| projection.map_projective(x, y);
    let wa = |x, y| projection.map_affine(x, y);
    let wt = |x, y| projection.map_translation(x, y);
    use Interpolation as I;
    use TransformationClass as TC;

    match (interpolation, projection.class) {
        (I::Nearest, TC::Translation) => warp_inner(out, wt, nn),
        (I::Nearest, TC::Affine) => warp_inner(out, wa, nn),
        (I::Nearest, TC::Projection) => warp_inner(out, wp, nn)
    }
}

// Work horse of all warp functions
// TODO: make faster by avoiding boundary checks in inner section of src image
fn warp_inner<Fc, Fi>(out: &mut Rgb565Image, mapping: Fc, get_pixel: Fi)
where
    Fc: Fn(f32, f32) -> (f32, f32) + Send + Sync,
    Fi: Fn(f32, f32) -> u16,
{
    let width = out.width;

    out.pixels.chunks_mut(width as usize).enumerate().for_each(|(y, row)| {
        for (x, p) in row.iter_mut().enumerate() {
            let (px, py) = mapping(x as f32, y as f32);
            *p = get_pixel(px, py);
        }
    });
}

// Classifies transformation by looking up transformation matrix coefficients
fn class_from_matrix(mx: [f32; 9]) -> TransformationClass {
    if (mx[6] - 0.0).abs() < 1e-10 && (mx[7] - 0.0).abs() < 1e-10 && (mx[8] - 1.0).abs() < 1e-10 {
        if (mx[0] - 1.0).abs() < 1e-10
            && (mx[1] - 0.0).abs() < 1e-10
            && (mx[3] - 0.0).abs() < 1e-10
            && (mx[4] - 1.0).abs() < 1e-10
        {
            TransformationClass::Translation
        } else {
            TransformationClass::Affine
        }
    } else {
        TransformationClass::Projection
    }
}

fn normalize(mx: [f32; 9]) -> [f32; 9] {
    [
        mx[0] / mx[8],
        mx[1] / mx[8],
        mx[2] / mx[8],
        mx[3] / mx[8],
        mx[4] / mx[8],
        mx[5] / mx[8],
        mx[6] / mx[8],
        mx[7] / mx[8],
        1.0,
    ]
}

// TODO: write me in f64
fn try_inverse(t: &[f32; 9]) -> Option<[f32; 9]> {
    let [t00, t01, t02, t10, t11, t12, t20, t21, t22] = t;

    let m00 = t11 * t22 - t12 * t21;
    let m01 = t10 * t22 - t12 * t20;
    let m02 = t10 * t21 - t11 * t20;

    let det = t00 * m00 - t01 * m01 + t02 * m02;

    if det.abs() < 1e-10 {
        return None;
    }

    let m10 = t01 * t22 - t02 * t21;
    let m11 = t00 * t22 - t02 * t20;
    let m12 = t00 * t21 - t01 * t20;
    let m20 = t01 * t12 - t02 * t11;
    let m21 = t00 * t12 - t02 * t10;
    let m22 = t00 * t11 - t01 * t10;

    #[rustfmt::skip]
    let inv = [
         m00 / det, -m10 / det,  m20 / det,
        -m01 / det,  m11 / det, -m21 / det,
         m02 / det, -m12 / det,  m22 / det,
    ];

    Some(normalize(inv))
}

fn mul3x3(a: [f32; 9], b: [f32; 9]) -> [f32; 9] {
    let [a00, a01, a02, a10, a11, a12, a20, a21, a22] = a;
    let [b00, b01, b02, b10, b11, b12, b20, b21, b22] = b;
    [
        a00 * b00 + a01 * b10 + a02 * b20,
        a00 * b01 + a01 * b11 + a02 * b21,
        a00 * b02 + a01 * b12 + a02 * b22,
        a10 * b00 + a11 * b10 + a12 * b20,
        a10 * b01 + a11 * b11 + a12 * b21,
        a10 * b02 + a11 * b12 + a12 * b22,
        a20 * b00 + a21 * b10 + a22 * b20,
        a20 * b01 + a21 * b11 + a22 * b21,
        a20 * b02 + a21 * b12 + a22 * b22,
    ]
}


#[inline(always)]
fn interpolate_nearest<'a>(image: &Rgb565Image<'a>, x: f32, y: f32, default: Rgb565Pixel) -> u16 {
    if x < -0.5 || y < -0.5 {
        return default.0;
    }

    let (width, height) = (image.width as u32, image.height as u32);

    let rx = (x + 0.5) as u32;
    let ry = (y + 0.5) as u32;

    if rx >= width || ry >= height {
        default.0
    } else {
        image.get_pixel(rx, ry).0
    }
}

/// How to handle pixels whose pre-image lies between input pixels.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Interpolation {
    /// Choose the nearest pixel to the pre-image of the
    /// output pixel.
    Nearest
}

pub struct Canvas<'a>{
    pub buf: &'a mut [u16],
    pub width: usize,
    pub height: usize,
}

impl <'a> Canvas<'a>{
    pub fn clear(&mut self, color: u16){
        self.buf.fill(color);
    }
    
    pub fn draw_image_at(&mut self, x: usize, y: usize, image:&[u16], image_width: usize){
        let mut begin = y*self.width + x;
        for row in image.chunks(image_width){
            let mut end = begin+image_width;
            if end > self.buf.len(){
                end = self.buf.len();                
            }
            let len = end - begin;
            self.buf[begin..end].copy_from_slice(&row[0..len]);
            begin += self.width;
        }
    }
}
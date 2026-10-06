//! The low-res dithered canvas behind the planet and the genome opening.

use macroquad::prelude::*;

pub const W: usize = 180;

pub type Rgb = [u8; 3];

const BAYER: [f32; 16] = [
    0.03, 0.53, 0.16, 0.66, 0.78, 0.28, 0.91, 0.41, 0.22, 0.72, 0.09, 0.59, 0.97, 0.47, 0.84, 0.34,
];

pub fn bayer(x: i32, y: i32) -> f32 {
    BAYER[((y & 3) << 2 | (x & 3)) as usize]
}

/// A stable pseudo-random value in `[0, 1]` per integer coordinate.
pub fn hash(x: i32, y: i32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) as f32) / u32::MAX as f32
}

/// `t * rate` wrapped into one turn, in f64 first: a sine of it stays
/// smooth however long the clock has been running.
pub fn phase(t: f64, rate: f64) -> f32 {
    ((t * rate) % std::f64::consts::TAU) as f32
}

pub fn mix(a: Rgb, b: Rgb, f: f32) -> Rgb {
    let f = f.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * f) as u8)
}

pub fn rgb_of(c: Color) -> Rgb {
    [
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
    ]
}

pub struct Canvas {
    pub h: usize,
    img: Image,
    tex: Texture2D,
    width: fn() -> f32,
}

impl Canvas {
    pub fn new() -> Self {
        Self::shown_at(screen_width)
    }

    pub fn in_column() -> Self {
        Self::shown_at(crate::gfx::view::width)
    }

    fn shown_at(width: fn() -> f32) -> Self {
        let h = Self::height_for(width);
        let img = Image::gen_image_color(W as u16, h as u16, BLACK);
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        Self { h, img, tex, width }
    }

    fn height_for(width: fn() -> f32) -> usize {
        ((screen_height() / width().max(1.0)) * W as f32)
            .round()
            .clamp(200.0, 480.0) as usize
    }

    pub fn fits_screen(&self) -> bool {
        Self::height_for(self.width) == self.h
    }

    /// Off-canvas writes are dropped.
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= W as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as usize * W + x as usize) * 4;
        self.img.bytes[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    }

    /// Off-canvas reads return the nearest edge pixel.
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        let (x, y) = (x.clamp(0, W as i32 - 1), y.clamp(0, self.h as i32 - 1));
        let i = (y as usize * W + x as usize) * 4;
        let d = &self.img.bytes;
        [d[i], d[i + 1], d[i + 2]]
    }

    pub fn present(&mut self) {
        self.tex.update(&self.img);
        draw_texture_ex(
            &self.tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2((self.width)(), screen_height())),
                ..Default::default()
            },
        );
    }
}

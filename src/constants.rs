use enum_map::{enum_map, Enum, EnumMap};
use image::{
    imageops, io::Reader as ImageReader, GenericImageView, GrayImage, ImageBuffer, ImageFormat,
    Luma, Rgb, Rgba, RgbaImage, SubImage,
};
use lazy_static::lazy_static;
use lru::LruCache;
use nanorand::{RandomGen, Rng};
use ndarray::{Array2, ArrayView2};
use piston_window::{G2dTexture, G2dTextureContext, PistonWindow, TextureSettings};
use std::{io::Cursor, num::NonZeroUsize};

pub const WINDOW_SIZE: (u32, u32) = (600, 576);
pub const BOARD_SIZE: (usize, usize) = (384, 576);
pub const BLOCK_SIZE: usize = 32;
pub const SAND_SIZE: usize = 4;
pub const SAND_BLOCK_SIZE: usize = BLOCK_SIZE / SAND_SIZE;
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const TEXT_COLOR: Rgba<u8> = Rgba([0, 0, 0, 255]);
pub const UI_BACKGROUND_COLOR: [f32; 4] = [89.0 / 255.0, 92.0 / 255.0, 102.0 / 255.0, 1.0];
pub const UI_ELEMENT_BG_COLOR: Rgba<u8> = Rgba([255, 255, 255, 255]);
pub const UI_ELEMENT_BG_COLOR_FLOAT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const MOVE_DELAY: f64 = 1.0 / 6.0;
pub const FIRST_INPUT_DELAY: f64 = 0.1;
pub const INPUT_DELAY: f64 = 1.0 / 60.0;
pub const MOVE_REPEAT: usize = 2;
pub const PHYSICS_DELAY: f64 = 1.0 / 30.0;
pub const FLASH_DELAY: f64 = 1.0 / 4.0;

pub const SCORE_Y: u32 = 192;
pub const SCORE_SCALE: usize = 4;
pub const SCORE_LABEL_SCALE: usize = 3;
pub const SCORE_DIGITS: usize = 6;
pub const NEXT_BLOCK_Y: u32 = 48;
pub const NEXT_BLOCK_DISPLAY_WIDTH: f64 = BLOCK_SIZE as f64 * 2.5;
pub const NEXT_BLOCK_DISPLAY_HEIGHT: f64 = BLOCK_SIZE as f64 * 2.5;
pub const NEXT_BLOCK_LABEL_SCALE: usize = 2;

#[rustfmt::skip]
lazy_static! {
    static ref SHAPES: EnumMap<Shape, Array2<bool>> = enum_map! {
        Shape::T => Array2::from_shape_vec([2, 3], vec![
            false, true, false,
            true , true, true ,
        ]).unwrap().reversed_axes(),
        Shape::S => Array2::from_shape_vec([2, 3], vec![
            false, true , true ,
            true , true , false,
        ]).unwrap().reversed_axes(),
        Shape::Z => Array2::from_shape_vec([2, 3], vec![
            true , true , false,
            false, true , true ,
        ]).unwrap().reversed_axes(),
        Shape::I => Array2::from_shape_vec([4, 1], vec![
            true , true , true , true ,
        ]).unwrap(),
        Shape::O => Array2::from_shape_vec([2, 2], vec![
            true , true ,
            true , true ,
        ]).unwrap(),
    };

    static ref PIXEL_FONT_SPRITES: GrayImage = ImageReader::with_format(
        Cursor::new(include_bytes!("../assets/font.png")),
        ImageFormat::Png,
    )
        .decode()
        .unwrap()
        .into_luma8();

    static ref ALPHA_CHARS: [SubImage<&'static GrayImage>; 26] =
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
            .map(|i| PIXEL_FONT_SPRITES.view(i * 5, 0, 5, 7));

    static ref NUMERIC_CHARS: [SubImage<&'static GrayImage>; 10] =
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
            .map(|i| PIXEL_FONT_SPRITES.view(i * 5, 7, 5, 7));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum Shape {
    // I,
    T,
    S,
    Z,
    I,
    O,
}

impl Shape {
    pub fn shape(&self) -> ArrayView2<'static, bool> {
        SHAPES[*self].view()
    }

    pub fn coords(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
        SHAPES[*self]
            .indexed_iter()
            .filter_map(move |((px, py), v)| {
                v.then_some((x + (px * SAND_BLOCK_SIZE), y + (py * SAND_BLOCK_SIZE)))
            })
    }
}

impl<Generator: Rng<OUTPUT>, const OUTPUT: usize> RandomGen<Generator, OUTPUT> for Shape {
    fn random(rng: &mut Generator) -> Self {
        [Shape::T, Shape::S, Shape::Z, Shape::I, Shape::O][rng.generate_range(0..5)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum Color {
    Red,
    Yellow,
    Blue,
    Green,
}

impl Color {
    const COLORS: EnumMap<Color, [u8; 4]> = EnumMap::from_array([
        [204, 0, 0, 255],
        [241, 194, 50, 255],
        [61, 133, 198, 255],
        [106, 168, 79, 255],
    ]);

    pub fn pixel_color(&self) -> Rgba<u8> {
        Rgba(Self::COLORS[*self])
    }

    pub fn float_color(&self) -> [f32; 4] {
        Self::COLORS[*self].map(|x| x as f32 / 255.0)
    }
}

impl<Generator: Rng<OUTPUT>, const OUTPUT: usize> RandomGen<Generator, OUTPUT> for Color {
    fn random(rng: &mut Generator) -> Self {
        [Color::Red, Color::Yellow, Color::Blue, Color::Green][rng.generate_range(0..4)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, enum_map::Enum)]
pub enum Direction {
    Left,
    Right,
    Down,
}

pub struct TextTextures {
    texture_context: G2dTextureContext,
    cache: LruCache<(String, usize, Rgba<u8>, Option<Rgba<u8>>), G2dTexture>,
}

impl TextTextures {
    pub fn new(window: &mut PistonWindow) -> Self {
        Self {
            texture_context: window.create_texture_context(),
            cache: LruCache::new(NonZeroUsize::new(64).unwrap()),
        }
    }

    pub fn texture(&mut self, text: &str, scale: usize, color: Rgba<u8>) -> Option<&G2dTexture> {
        self.cache
            .try_get_or_insert((text.to_string(), scale, color, None), || {
                G2dTexture::from_image(
                    &mut self.texture_context,
                    &Self::generate_text_sprite(text, scale, color, None).ok_or(())?,
                    &TextureSettings::new(),
                )
                .map_err(|_| ())
            })
            .ok()
    }

    pub fn texture_with_background(
        &mut self,
        text: &str,
        scale: usize,
        color: Rgba<u8>,
        background: Rgba<u8>,
    ) -> Option<&G2dTexture> {
        self.cache
            .try_get_or_insert((text.to_string(), scale, color, Some(background)), || {
                G2dTexture::from_image(
                    &mut self.texture_context,
                    &Self::generate_text_sprite(text, scale, color, Some(background)).ok_or(())?,
                    &TextureSettings::new(),
                )
                .map_err(|_| ())
            })
            .ok()
    }

    fn get_sprite(c: char) -> Option<SubImage<&'static GrayImage>> {
        match c {
            'A'..='Z' => Some(ALPHA_CHARS[c as usize - 'A' as usize]),
            'a'..='z' => Some(ALPHA_CHARS[c as usize - 'a' as usize]),
            '0'..='9' => Some(NUMERIC_CHARS[c as usize - '0' as usize]),
            _ => None,
        }
    }

    fn generate_text_sprite(
        text: &str,
        scale: usize,
        color: Rgba<u8>,
        background: Option<Rgba<u8>>,
    ) -> Option<RgbaImage> {
        let offset = if background.is_some() { 1 } else { 0 };
        let width = (text.len() * 5 + text.len() - 1) as u32 + offset * 2;
        let mut buffer = GrayImage::from_pixel(width, 7 + offset * 2, Luma([255]));
        for (i, c) in text.chars().enumerate() {
            if c != ' ' {
                imageops::replace(
                    &mut buffer,
                    &*Self::get_sprite(c)?,
                    i as i64 * 6 + offset as i64,
                    offset as i64,
                );
            }
        }

        let Rgba([text_r, text_g, text_b, text_a]) = color;
        let Rgba([bg_r, bg_g, bg_b, bg_a]) = background.unwrap_or(Rgba([0, 0, 0, 0]));
        let colored_buffer = imageproc::map::map_colors(&buffer, |Luma([scale])| {
            Rgba([
                scale_subpixel(text_r, bg_r, scale),
                scale_subpixel(text_g, bg_g, scale),
                scale_subpixel(text_b, bg_b, scale),
                scale_subpixel(text_a, bg_a, scale),
            ])
        });

        Some(imageops::resize(
            &colored_buffer,
            width * scale as u32,
            (7 + offset * 2) * scale as u32,
            imageops::FilterType::Nearest,
        ))
    }
}

fn scale_subpixel(text: u8, background: u8, scale: u8) -> u8 {
    let scale = scale as u16;
    ((text as u16 * (255 - scale) + background as u16 * scale) / 255) as u8
}

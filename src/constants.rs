use enum_map::{enum_map, Enum, EnumMap};
use image::Rgba;
use lazy_static::lazy_static;
use nanorand::{RandomGen, Rng};
use ndarray::{Array2, ArrayView2};

pub const BLOCK_SIZE: usize = 32;
pub const SAND_SIZE: usize = 4;
pub const SAND_BLOCK_SIZE: usize = BLOCK_SIZE / SAND_SIZE;
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const MOVE_DELAY: f64 = 1.0 / 6.0;
pub const FIRST_INPUT_DELAY: f64 = 0.1;
pub const INPUT_DELAY: f64 = 1.0 / 60.0;
pub const MOVE_REPEAT: usize = 2;
pub const PHYSICS_DELAY: f64 = 1.0 / 30.0;
pub const FLASH_DELAY: f64 = 1.0 / 4.0;

pub const PIXEL_FONT: &'static [u8] = include_bytes!("../assets/Minimal3x5.ttf");

#[rustfmt::skip]
lazy_static! {
    static ref SHAPES: EnumMap<Shape, Array2<bool>> = dbg!(enum_map! {
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
    });
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

use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use nanorand::{Rng, RandomGen};
use ndarray::{Array2, ArrayView2};

pub const BLOCK_SIZE: usize = 32;
pub const SAND_SIZE: usize = 2;
pub const SAND_BLOCK_SIZE: usize = BLOCK_SIZE / SAND_SIZE;
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const MOVE_DELAY: f64 = 1.0 / 6.0;
pub const FIRST_INPUT_DELAY: f64 = 0.1;
pub const INPUT_DELAY: f64 = 1.0 / 60.0;
pub const MOVE_REPEAT: usize = 2;
pub const PHYSICS_DELAY: f64 = 1.0 / 60.0;

#[rustfmt::skip]
lazy_static! {
    static ref SHAPES: EnumMap<Shape, Array2<bool>> = dbg!(enum_map! {
        // Block::I => Array2::from_shape_vec([4, 1], vec![
        //     true , true , true , true ,
        // ]).unwrap(),
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
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum Shape {
    // I,
    T,
    S,
    Z,
}

impl Shape {
    pub const BLOCKS: [Shape; 3] = [Self::T, Self::S, Self::Z];

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
        [Shape::T, Shape::S, Shape::Z][rng.generate_range(0..3)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, enum_map::Enum)]
pub enum Direction {
    Left,
    Right,
    Down,
}

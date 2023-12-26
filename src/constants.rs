use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
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
    static ref BLOCKS: EnumMap<Block, Array2<bool>> = dbg!(enum_map! {
        // Block::I => Array2::from_shape_vec([4, 1], vec![
        //     true , true , true , true ,
        // ]).unwrap(),
        Block::T => Array2::from_shape_vec([2, 3], vec![
            false, true, false,
            true , true, true ,
        ]).unwrap().reversed_axes(),
        Block::S => Array2::from_shape_vec([2, 3], vec![
            false, true , true ,
            true , true , false,
        ]).unwrap().reversed_axes(),
        Block::Z => Array2::from_shape_vec([2, 3], vec![
            true , true , false,
            false, true , true ,
        ]).unwrap().reversed_axes(),
    });
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Enum)]
pub enum Block {
    // I,
    T,
    S,
    Z,
}

impl Block {
    pub const BLOCKS: [Block; 3] = [Self::T, Self::S, Self::Z];

    pub fn shape(&self) -> ArrayView2<'static, bool> {
        BLOCKS[*self].view()
    }

    pub fn coords(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
        BLOCKS[*self]
            .indexed_iter()
            .filter_map(move |((px, py), v)| {
                v.then_some((x + (px * SAND_BLOCK_SIZE), y + (py * SAND_BLOCK_SIZE)))
            })
    }
}

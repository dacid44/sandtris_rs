use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use ndarray::Array2;

#[rustfmt::skip]
lazy_static! {
    pub static ref BLOCKS: EnumMap<Block, Array2<bool>> = enum_map! {
        // Block::I => Array2::from_shape_vec([4, 1], vec![
        //     true , true , true , true ,
        // ]).unwrap(),
        Block::T => Array2::from_shape_vec([3, 2], vec![
            false, true, false,
            true , true, true ,
        ]).unwrap(),
        Block::S => Array2::from_shape_vec([3, 2], vec![
            false, true , true ,
            true , true , false,
        ]).unwrap(),
        Block::Z => Array2::from_shape_vec([3, 2], vec![
            true , true , false,
            false, true , true ,
        ]).unwrap(),
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Enum)]
pub enum Block {
    // I,
    T,
    S,
    Z,
}

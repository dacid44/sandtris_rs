use std::iter::once;

use nanorand::{Rng, WyRand};
use ndarray::{Array1, ArrayView1, ArrayView2, ArrayViewMut2};

use crate::constants::Direction;

pub fn run_physics_line(rng: &mut WyRand, sand: ArrayView2<bool>) -> Array1<Option<Direction>> {
    // Figure out what each grain of sand "wants to" do
    let requests = once(sand[[0, 0]]
        .then(|| [true, sand[[1, 1]], sand[[2, 1]]]))
        .chain(
            sand.windows([3, 2])
                .into_iter()
                .map(|w| w[[1, 0]].then(|| [w[[0, 1]], w[[1, 1]], w[[2, 1]]])),
        )
        .chain(
            once(sand[[sand.dim().0 - 1, 0]]
                .then(|| [sand[[sand.dim().0 - 2, 1]], sand[[sand.dim().0 - 1, 1]], true]))
        )
        .map(|o| o.and_then(|s| decide_direction(rng, s)))
        .collect::<Vec<_>>();

    // Resolve conflicts, first between neighboring sand grains (one will be straight down, one
    // will be diagonal. DIagonal gets priority)
    
    // Resolve conflicts between sand grains 
    todo!()
}

pub fn run_physics_line_2(rng: &mut WyRand, sand: ArrayViewMut2<bool>) {
    
}

fn decide_direction(rng: &mut WyRand, sand_under: [bool; 3]) -> Option<Direction> {
    use Direction as D;
    match sand_under {
        [true, true, true] => None,
        [false, true, true] => Some(D::Left),
        [true, false, true] => Some(D::Down),
        [true, true, false] => Some(D::Right),
        [false, true, false] => Some(if rng.generate() { D::Left } else { D::Right }),
        [left, _, right] => {
            let mut choices = [D::Down; 5];
            if !left {
                choices[0] = D::Left;
            }
            if !right {
                choices[4] = D::Right;
            }
            Some(choices[rng.generate_range(0..5)])
        }
    }
}

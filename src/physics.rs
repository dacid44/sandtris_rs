use std::iter::once;

use nanorand::{Rng, WyRand};
use ndarray::{s, Array1, ArrayView1, ArrayView2, ArrayViewMut2};

use crate::constants::Direction;

pub fn run_rng_physics<T>(rng: &mut WyRand, mut sand: ArrayViewMut2<Option<T>>) {
    for i in (1..sand.dim().1).rev() {
        for (j, m) in run_physics_line(rng, sand.slice(s![.., i - 1..=i]))
            .into_iter()
            .enumerate()
            .filter_map(|(j, m)| m.map(|m| (j, m)))
        {
            match m {
                Direction::Left => {
                    sand[[j - 1, i]] = sand[[j, i - 1]].take();
                }
                Direction::Right => {
                    sand[[j + 1, i]] = sand[[j, i - 1]].take();
                }
                Direction::Down => {
                    sand[[j, i]] = sand[[j, i - 1]].take();
                }
            };
        }
    }
}

pub fn run_physics_line<T>(
    rng: &mut WyRand,
    sand: ArrayView2<Option<T>>,
) -> Vec<Option<Direction>> {
    // Figure out what each grain of sand "wants to" do
    // assume that if there is a grain of sand next to the current one, the current one cannot move
    // diagonally in that direction
    let mut requests = once(sand[[0, 0]].is_some().then(|| {
        [
            true,
            sand[[1, 1]].is_some(),
            sand[[2, 1]].is_some() || sand[[2, 0]].is_some(),
        ]
    }))
    .chain(sand.windows([3, 2]).into_iter().map(|w| {
        w[[1, 0]].is_some().then(|| {
            [
                w[[0, 1]].is_some() || w[[0, 0]].is_some(),
                w[[1, 1]].is_some(),
                w[[2, 1]].is_some() || w[[2, 0]].is_some(),
            ]
        })
    }))
    .chain(once(sand[[sand.dim().0 - 1, 0]].is_some().then(|| {
        [
            sand[[sand.dim().0 - 2, 1]].is_some(),
            sand[[sand.dim().0 - 1, 1]].is_some(),
            true,
        ]
    })))
    .map(|o| o.and_then(|s| decide_direction(rng, s)))
    .collect::<Vec<_>>();

    // Resolve conflicts, first between neighboring sand grains (one will be straight down, one
    // will be diagonal. Straight down gets priority)
    let mut changed = true;
    while changed {
        // Loop until all conflicts have been resolved
        changed = false;
        // Resolve conflicts of neighboring sand grains
        for (left, right) in (1..requests.len()).map(|i| (i - 1, i)) {
            match (requests[left], requests[right]) {
                (Some((Direction::Right, next)), Some((Direction::Down, _))) => {
                    requests[left] = next.map(|next| (next, None));
                    changed = true;
                }
                (Some((Direction::Down, _)), Some((Direction::Left, next))) => {
                    requests[right] = next.map(|next| (next, None));
                    changed = true;
                }
                _ => {}
            }
        }

        for (left, right) in (2..requests.len()).map(|i| (i - 2, i)) {
            if let (Some((Direction::Right, l_next)), Some((Direction::Left, r_next))) =
                (requests[left], requests[right])
            {
                changed = true;
                if rng.generate() {
                    requests[left] = l_next.map(|next| (next, None));
                } else {
                    requests[right] = r_next.map(|next| (next, None));
                }
            }
        }
    }

    requests
        .into_iter()
        .map(|request| request.map(|d| d.0))
        .collect()
}

fn decide_direction(
    rng: &mut WyRand,
    sand_under: [bool; 3],
) -> Option<(Direction, Option<Direction>)> {
    use Direction as D;
    match sand_under {
        [true, true, true] => None,
        [false, true, true] => Some((D::Left, None)),
        [true, false, true] => Some((D::Down, None)),
        [true, true, false] => Some((D::Right, None)),
        [false, true, false] => Some(if rng.generate() {
            (D::Left, Some(D::Right))
        } else {
            (D::Right, Some(D::Left))
        }),
        [left, false, right] => {
            const CHOICES_LEN: usize = 32;
            let mut choices = [(D::Down, None); CHOICES_LEN];
            if !left {
                choices[0] = (D::Left, Some(D::Down));
            }
            if !right {
                choices[CHOICES_LEN - 1] = (D::Right, Some(D::Down));
            }
            Some(choices[rng.generate_range(0..CHOICES_LEN)])
        }
    }
}

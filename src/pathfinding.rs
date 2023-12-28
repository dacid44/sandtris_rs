use std::iter;

use ndarray::{s, Array2, ArrayView2};
use pathfinding::directed::astar::astar;

use crate::constants::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Node {
    StartingEdge,
    Grid(usize, usize),
}

fn find_spanning_group(grid: &Array2<Option<Color>>) -> [usize; 2] {
    let path = astar(
        &Node::StartingEdge,
        |node| match node {
            Node::StartingEdge => Box::new(
                grid.slice(s![0, ..])
                    .indexed_iter()
                    .flat_map(|((x, y), color)| color.map(|_| (Node::Grid(x, y), 1))),
            ),
            Node::Grid(x, y) => {
                let Some(color) = grid[[x, y]] else {
                    return Box::new(iter::empty());
                };
                Box::new(
                    [
                        (x.wrapping_sub(1), *y),
                        (*x, y.wrapping_sub(1)),
                        (x + 1, *y),
                        (*x, y + 1),
                    ]
                    .into_iter()
                    .flat_map(|(nx, ny)| test_node(grid, nx, ny, color)),
                )
            }
        },
        |node| match node {
            Node::StartingEdge => grid.dim().0,
            Node::Grid(x, _) => grid.dim().0 - 1 - x,
        },
        |node| match node {
            Node::StartingEdge => false,
            Node::Grid(x, _) => x == grid.dim().0 - 1,
        },
    );
    todo!()
}

fn test_node(
    grid: &Array2<Option<Color>>,
    x: usize,
    y: usize,
    color: Color,
) -> Option<(Node, usize)> {
    grid.get([x, y])
        .copied()
        .flatten()
        .filter(|c| c == color)
        .map(|_| (Node::Grid(x, y), 1))
}

use std::iter;

use ndarray::{s, Array, Array1, Array2, ArrayView2};
use pathfinding::directed::{astar::astar, bfs::bfs_reach};

use crate::constants::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    StartingEdge,
    Grid(usize, usize),
}

pub fn find_spanning_group(grid: &Array2<Option<Color>>) -> Option<(usize, usize)> {
    astar(
        &Node::StartingEdge,
        |node| -> Box<dyn Iterator<Item = (Node, usize)>> {
            match node {
                Node::StartingEdge => Box::new(
                    (0..grid.dim().1).filter_map(|y| grid[[0, y]].map(|_| (Node::Grid(0, y), 1))),
                ),
                Node::Grid(x, y) => {
                    if let Some(color) = grid[[*x, *y]] {
                        Box::new(find_neighbors(grid, *x, *y, color).map(|(nx, ny)| (Node::Grid(nx, ny), 1)))
                    } else {
                        Box::new(iter::empty())
                    }
                }
            }
        },
        |node| match node {
            Node::StartingEdge => grid.dim().0,
            Node::Grid(x, _) => grid.dim().0 - 1 - x,
        },
        |node| match node {
            Node::StartingEdge => false,
            Node::Grid(x, _) => *x == grid.dim().0 - 1,
        },
    )
    .and_then(|path| match path.0[1] {
        Node::StartingEdge => None,
        Node::Grid(x, y) => Some((x, y)),
    })
}

pub fn find_connected_sand(grid: &Array2<Option<Color>>, x: usize, y: usize) -> Vec<(usize, usize)> {
    bfs_reach((x, y), |(x, y)| -> Box<dyn Iterator<Item=(usize, usize)>> {
        if let Some(color) = grid[[*x, *y]] {
            Box::new(find_neighbors(grid, *x, *y, color))
        } else {
            Box::new(iter::empty())
        }
    }).collect()
}

fn find_neighbors(
    grid: &Array2<Option<Color>>,
    x: usize,
    y: usize,
    color: Color,
) -> impl Iterator<Item = (usize, usize)> {
    [
        (x.wrapping_sub(1), y),
        (x, y.wrapping_sub(1)),
        (x + 1, y),
        (x, y + 1),
    ]
    .map(|(nx, ny)| test_node(grid, nx, ny, color))
    .into_iter()
    .flatten()
}

fn test_node(grid: &Array2<Option<Color>>, x: usize, y: usize, color: Color) -> Option<(usize, usize)> {
    grid.get([x, y])
        .copied()
        .flatten()
        .filter(|c| *c == color)
        .map(|_| (x, y))
}

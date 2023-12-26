use crate::canvas::Canvas;
use crate::constants::*;
use enum_map::EnumMap;
use image::Rgba;
use imageproc::drawing;
use imageproc::rect::Rect;
use nanorand::Rng;
use nanorand::WyRand;
use ndarray::s;
use ndarray::Array;
use ndarray::Array2;
use piston_window::graphics;
use piston_window::prelude::*;

#[derive(Debug)]
pub struct Game {
    rng: WyRand,
    canvas: Canvas,
    sand: Array2<bool>,
    elapsed_time: f64,
    next_move: f64,
    next_physics_update: f64,
    control_updates: EnumMap<Direction, Option<f64>>,
    falling_block_pos: Option<(usize, usize, Block)>,
}

impl Game {
    pub fn new(window: &mut PistonWindow) -> Self {
        Self {
            rng: WyRand::new(),
            canvas: Canvas::new(window),
            sand: Array2::default([
                window.size().width as usize / SAND_SIZE,
                window.size().height as usize / SAND_SIZE,
            ]),
            elapsed_time: 0.0,
            next_move: MOVE_DELAY,
            next_physics_update: 0.0,
            control_updates: Default::default(),
            falling_block_pos: None,
        }
    }

    fn reset(&mut self) {
        self.sand.assign(&Array::from_elem(1, false));
        self.next_move = self.elapsed_time + MOVE_DELAY;
        self.next_physics_update = self.elapsed_time;
        self.falling_block_pos = None;
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Some(button) = event.press_args() {
            match button {
                Button::Keyboard(key) => match key {
                    Key::Left => {
                        self.move_block(Direction::Left);
                        self.control_updates[Direction::Left] =
                            Some(self.elapsed_time + FIRST_INPUT_DELAY);
                    }
                    Key::Right => {
                        self.move_block(Direction::Right);
                        self.control_updates[Direction::Right] =
                            Some(self.elapsed_time + FIRST_INPUT_DELAY);
                    }
                    Key::Down => {
                        self.move_block(Direction::Down);
                        self.control_updates[Direction::Down] =
                            Some(self.elapsed_time + FIRST_INPUT_DELAY);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if let Some(button) = event.release_args() {
            match button {
                Button::Keyboard(key) => match key {
                    Key::Left => {
                        self.control_updates[Direction::Left] = None;
                    }
                    Key::Right => {
                        self.control_updates[Direction::Right] = None;
                    }
                    Key::Down => {
                        self.control_updates[Direction::Down] = None;
                    }
                    Key::R => {
                        self.reset();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn move_block(&mut self, direction: Direction) {
        for _ in 0..MOVE_REPEAT {
            match direction {
                Direction::Left => {
                    if let Some((x, y, block)) = self
                        .falling_block_pos
                        .filter(|_| self.can_move(Direction::Left))
                    {
                        self.falling_block_pos = Some((x - 1, y, block));
                    }
                }
                Direction::Right => {
                    if let Some((x, y, block)) = self
                        .falling_block_pos
                        .filter(|_| self.can_move(Direction::Right))
                    {
                        self.falling_block_pos = Some((x + 1, y, block));
                    }
                }
                Direction::Down => {
                    if let Some((x, y, block)) = self.falling_block_pos {
                        if self.can_move(Direction::Down) {
                            self.falling_block_pos = Some((x, y + 1, block))
                        } else {
                            self.add_sand_block();
                            self.falling_block_pos = None;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, event: &UpdateArgs) {
        self.elapsed_time += event.dt;

        self.control_updates = self.control_updates.map(|input, update| {
            if let Some(update) = update.filter(|update| self.elapsed_time >= *update) {
                self.move_block(input);
                Some(update + INPUT_DELAY)
            } else {
                update
            }
        });

        if self.elapsed_time >= self.next_physics_update {
            self.run_sand_physics();
            self.next_physics_update += PHYSICS_DELAY;
        }

        if self.elapsed_time >= self.next_move {
            if self.falling_block_pos.is_some() {
                if self.control_updates[Direction::Down].is_none() {
                    self.move_block(Direction::Down);
                }
            } else {
                self.falling_block_pos = Some((
                    self.sand.dim().0 / 2 - 1,
                    0,
                    Block::BLOCKS[self.rng.generate_range(0..Block::BLOCKS.len())],
                ));
            }
            self.next_move += MOVE_DELAY;
        }
    }

    fn is_block_settled(&self) -> bool {
        // self.falling_block_pos
        //     .map(|(x, y)| y + Self::SAND_BLOCK_SIZE == self.sand.dim().1)
        //     .unwrap_or(false)
        !self.can_move(Direction::Down)
    }

    fn can_move(&self, direction: Direction) -> bool {
        if let Some((x, y, block)) = self.falling_block_pos {
            match direction {
                Direction::Left => {
                    // TODO: Check sand
                    x > 0 && block.coords(x, y).all(|(px, py)| {
                        self.sand.slice(s![px - 1, py..py + SAND_BLOCK_SIZE]).iter().all(|s| !s)
                    })
                }
                Direction::Right => {
                    // TODO: Check sand
                    x < self.sand.dim().0 - (SAND_BLOCK_SIZE * block.shape().dim().0)
                        && block.coords(x, y).all(|(px, py)| {
                        self.sand.slice(s![px + SAND_BLOCK_SIZE, py..py + SAND_BLOCK_SIZE]).iter().all(|s| !s)
                    })
                }
                Direction::Down => {
                    y < self.sand.dim().1 - (SAND_BLOCK_SIZE * block.shape().dim().1)
                        && block.coords(x, y).all(|(px, py)| {
                            self.sand
                                .slice(s![px..px + SAND_BLOCK_SIZE, py + SAND_BLOCK_SIZE])
                                .iter()
                                .all(|s| !s)
                        })
                }
            }
        } else {
            false
        }
    }

    fn add_sand_block(&mut self) {
        if let Some((x, y, block)) = self.falling_block_pos {
            for (px, py) in block.coords(x, y) {
                self.sand
                    .slice_mut(s![px..px + SAND_BLOCK_SIZE, py..py + SAND_BLOCK_SIZE])
                    .assign(&Array::from_elem(1, true));
            }
        }
    }

    fn run_sand_physics(&mut self) {
        // The bottom line will not move so we can skip it, and not worry about the bottom edge
        // case
        for y in (0..self.sand.dim().1 - 1).rev() {
            for x in 0..self.sand.dim().0 {
                if !self.sand[[x, y]] {
                    continue;
                }

                // Check directly below
                if !self.sand[[x, y + 1]] {
                    self.sand[[x, y + 1]] = true;
                    self.sand[[x, y]] = false;
                    continue;
                }
            }

            for x in 0..self.sand.dim().0 {
                if !self.sand[[x, y]] {
                    continue;
                }

                // TODO: Make which direction the sand actually goes randomly decided

                // Check bottom left
                if x > 0 && !self.sand[[x - 1, y + 1]] {
                    self.sand[[x - 1, y + 1]] = true;
                    self.sand[[x, y]] = false;
                    continue;
                }

                // Check bottom right
                if x < self.sand.dim().0 - 1 && !self.sand[[x + 1, y + 1]] {
                    self.sand[[x + 1, y + 1]] = true;
                    self.sand[[x, y]] = false;
                    continue;
                }
            }
        }
    }

    fn draw_block(
        &self,
        x: usize,
        y: usize,
        color: [f32; 4],
        context: graphics::Context,
        g: &mut G2d,
    ) {
        let (x, y) = (x as f64, y as f64);
        graphics::rectangle_from_to(
            color,
            [x, y],
            [x + BLOCK_SIZE as f64, y + BLOCK_SIZE as f64],
            context.transform,
            g,
        );
    }

    pub fn render(&mut self, context: graphics::Context, g: &mut G2d) {
        self.canvas.clear(Rgba([255, 255, 255, 255]));
        let buffer = self.canvas.image();

        graphics::clear(CLEAR_COLOR, g);
        for ((x, y), _) in self.sand.indexed_iter().filter(|(_, pixel)| **pixel) {
            // buffer.put_pixel(x as u32, y as u32, Rgba([0, 255, 0, 255]));
            drawing::draw_filled_rect_mut(
                buffer,
                Rect::at((x * SAND_SIZE) as i32, (y * SAND_SIZE) as i32)
                    .of_size(SAND_SIZE as u32, SAND_SIZE as u32),
                Rgba([0, 255, 0, 255]),
            );
        }
        self.canvas.render(context, g);

        // for ((x, y), _) in self.blocks.indexed_iter().filter(|(_, x)| **x) {
        //     self.draw_block(x, y, [1.0, 0.0, 0.0, 1.0], context, g);
        // }

        if let Some((x, y, block)) = self.falling_block_pos {
            for (px, py) in block.coords(x, y) {
                self.draw_block(
                    px * SAND_SIZE,
                    py * SAND_SIZE,
                    [0.0, 0.0, 1.0, 1.0],
                    context,
                    g,
                );
            }
        }
    }
}

// #[derive(Debug, Default)]
// struct ControlUpdates {
//     left: Option<f64>,
//     right: Option<f64>,
//     down: Option<f64>,
// }

#[derive(Debug, PartialEq, Eq, enum_map::Enum)]
enum Direction {
    Left,
    Right,
    Down,
}

use crate::canvas::Canvas;
use enum_map::EnumMap;
use image::Rgba;
use imageproc::drawing;
use imageproc::rect::Rect;
use nanorand::Rng;
use ndarray::s;
use ndarray::Array;
use ndarray::Array2;
use piston_window::graphics;
use piston_window::prelude::*;

#[derive(Debug)]
pub struct Game {
    canvas: Canvas,
    sand: Array2<bool>,
    elapsed_time: f64,
    next_move: f64,
    next_physics_update: f64,
    control_updates: EnumMap<Direction, Option<f64>>,
    falling_block_pos: Option<(usize, usize)>,
}

impl Game {
    const BLOCK_SIZE: usize = 32;
    const SAND_SIZE: usize = 2;
    const SAND_BLOCK_SIZE: usize = Self::BLOCK_SIZE / Self::SAND_SIZE;
    const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    const MOVE_DELAY: f64 = 1.0 / 6.0;
    const FIRST_INPUT_DELAY: f64 = 0.1;
    const INPUT_DELAY: f64 = 1.0 / 60.0;
    const MOVE_REPEAT: usize = 2;
    const PHYSICS_DELAY: f64 = 1.0 / 60.0;

    pub fn new(window: &mut PistonWindow) -> Self {
        Self {
            canvas: Canvas::new(window),
            sand: Array2::default([
                window.size().width as usize / Self::SAND_SIZE,
                window.size().height as usize / Self::SAND_SIZE,
            ]),
            elapsed_time: 0.0,
            next_move: Self::MOVE_DELAY,
            next_physics_update: 0.0,
            control_updates: Default::default(),
            falling_block_pos: None,
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Some(button) = event.press_args() {
            match button {
                Button::Keyboard(key) => match key {
                    Key::Left => {
                        self.move_block(Direction::Left);
                        self.control_updates[Direction::Left] =
                            Some(self.elapsed_time + Self::FIRST_INPUT_DELAY);
                    }
                    Key::Right => {
                        self.move_block(Direction::Right);
                        self.control_updates[Direction::Right] =
                            Some(self.elapsed_time + Self::FIRST_INPUT_DELAY);
                    }
                    Key::Down => {
                        self.move_block(Direction::Down);
                        self.control_updates[Direction::Down] =
                            Some(self.elapsed_time + Self::FIRST_INPUT_DELAY);
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
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn move_block(&mut self, direction: Direction) {
        for _ in 0..Self::MOVE_REPEAT {
            match direction {
                Direction::Left => {
                    if let Some((x, _)) = self.falling_block_pos.as_mut().filter(|pos| pos.0 > 0) {
                        *x -= 1;
                    }
                }
                Direction::Right => {
                    if let Some((x, _)) = self
                        .falling_block_pos
                        .as_mut()
                        .filter(|pos| pos.0 < self.sand.dim().0 - Self::SAND_BLOCK_SIZE)
                    {
                        *x += 1;
                    }
                }
                Direction::Down => {
                    if let Some((x, y)) = self.falling_block_pos {
                        if self.is_block_settled() {
                            self.add_sand_block(x, y);
                            self.falling_block_pos = None;
                            break;
                        } else {
                            self.falling_block_pos = Some((x, y + 1))
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
                Some(update + Self::INPUT_DELAY)
            } else {
                update
            }
        });

        if self.elapsed_time >= self.next_physics_update {
            self.run_sand_physics();
            self.next_physics_update += Self::PHYSICS_DELAY;
        }

        if self.elapsed_time >= self.next_move {
            if let Some((x, y)) = self.falling_block_pos {
                if self.control_updates[Direction::Down].is_none() {
                    self.move_block(Direction::Down);
                }
            } else {
                self.falling_block_pos = Some((self.sand.dim().0 / 2, 0));
            }
            self.next_move += Self::MOVE_DELAY;
        }
    }

    fn is_block_settled(&self) -> bool {
        // self.falling_block_pos
        //     .map(|(x, y)| y + Self::SAND_BLOCK_SIZE == self.sand.dim().1)
        //     .unwrap_or(false)
        !self.can_move(Direction::Down)
    }

    fn can_move(&self, direction: Direction) -> bool {
        if let Some((x, y)) = self.falling_block_pos {
            match direction {
                Direction::Left => {
                    true
                }
                Direction::Right => {
                    true
                }
                Direction::Down => {
                    y < self.sand.dim().1 - Self::SAND_BLOCK_SIZE
                        && self.sand.slice(s![x..x + Self::SAND_BLOCK_SIZE, y + Self::SAND_BLOCK_SIZE]).iter().all(|s| !s)
                }
            }
        } else {
            false
        }
    }

    fn add_sand_block(&mut self, x: usize, y: usize) {
        self.sand
            .slice_mut(s![
                x..x + (Self::SAND_BLOCK_SIZE),
                y..y + (Self::SAND_BLOCK_SIZE)
            ])
            .assign(&Array::from_elem(1, true));
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
            [x + Self::BLOCK_SIZE as f64, y + Self::BLOCK_SIZE as f64],
            context.transform,
            g,
        );
    }

    pub fn render(&mut self, context: graphics::Context, g: &mut G2d) {
        self.canvas.clear(Rgba([255, 255, 255, 255]));
        let buffer = self.canvas.image();

        graphics::clear(Self::CLEAR_COLOR, g);
        for ((x, y), _) in self.sand.indexed_iter().filter(|(_, pixel)| **pixel) {
            // buffer.put_pixel(x as u32, y as u32, Rgba([0, 255, 0, 255]));
            drawing::draw_filled_rect_mut(
                buffer,
                Rect::at((x * Self::SAND_SIZE) as i32, (y * Self::SAND_SIZE) as i32)
                    .of_size(Self::SAND_SIZE as u32, Self::SAND_SIZE as u32),
                Rgba([0, 255, 0, 255]),
            );
        }
        self.canvas.render(context, g);

        // for ((x, y), _) in self.blocks.indexed_iter().filter(|(_, x)| **x) {
        //     self.draw_block(x, y, [1.0, 0.0, 0.0, 1.0], context, g);
        // }

        if let Some((x, y)) = self.falling_block_pos {
            self.draw_block(
                x * Self::SAND_SIZE,
                y * Self::SAND_SIZE,
                [0.0, 0.0, 1.0, 1.0],
                context,
                g,
            );
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

use crate::canvas::Canvas;
use enum_map::EnumMap;
use image::Rgba;
use nanorand::Rng;
use ndarray::Array2;
use piston_window::graphics;
use piston_window::prelude::*;

#[derive(Debug)]
pub struct Game {
    canvas: Canvas,
    blocks: Array2<bool>,
    sand: Array2<bool>,
    elapsed_time: f64,
    next_move: f64,
    control_updates: EnumMap<Direction, Option<f64>>,
    falling_block_pos: Option<(usize, usize)>,
}

impl Game {
    const BLOCK_SIZE: f64 = 32.0;
    const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    const MOVE_DELAY: f64 = 0.5;
    const FIRST_INPUT_DELAY: f64 = 0.1;
    const INPUT_DELAY: f64 = 0.05;

    pub fn new(window: &mut PistonWindow) -> Self {
        Self {
            canvas: Canvas::new(window),
            blocks: Array2::default([12, 16]),
            sand: Array2::default([window.size().width as usize, window.size().height as usize]),
            elapsed_time: 0.0,
            next_move: Self::MOVE_DELAY,
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
                            Some(self.elapsed_time + Self::FIRST_INPUT_DELAY)
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
                    .filter(|pos| pos.0 < self.blocks.dim().0 - 1)
                {
                    *x += 1;
                }
            }
            Direction::Down => {
                let is_settled = self.is_block_settled();
                if let Some((_, y)) = self.falling_block_pos.as_mut().filter(|_| !is_settled) {
                    *y += 1
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

        if self.elapsed_time >= self.next_move {
            let is_settled = self.is_block_settled();
            let is_down_held = self.control_updates[Direction::Down].is_some();
            if let Some((x, y)) = self.falling_block_pos.as_mut() {
                if is_settled {
                    self.blocks[(*x, *y)] = true;
                    self.falling_block_pos = None;
                } else if !is_down_held {
                    *y += 1;
                }
            } else {
                self.falling_block_pos = Some((self.blocks.dim().0 / 2, 0));
            }
            self.next_move += Self::MOVE_DELAY;
        }
    }

    fn is_block_settled(&self) -> bool {
        self.falling_block_pos
            .map(|(x, y)| y + 1 == self.blocks.dim().1 || self.blocks[(x, y + 1)])
            .unwrap_or(false)
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
            [x * Self::BLOCK_SIZE, y * Self::BLOCK_SIZE],
            [(x + 1.0) * Self::BLOCK_SIZE, (y + 1.0) * Self::BLOCK_SIZE],
            context.transform,
            g,
        );
    }

    pub fn render(&mut self, context: graphics::Context, g: &mut G2d) {
        self.canvas.clear(Rgba([255, 255, 255, 255]));
        let buffer = self.canvas.image();

        graphics::clear(Self::CLEAR_COLOR, g);

        for ((x, y), _) in self.blocks.indexed_iter().filter(|(_, x)| **x) {
            self.draw_block(x, y, [1.0, 0.0, 0.0, 1.0], context, g);
        }

        if let Some((x, y)) = self.falling_block_pos {
            self.draw_block(x, y, [0.0, 0.0, 1.0, 1.0], context, g);
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

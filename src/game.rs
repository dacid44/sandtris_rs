use crate::canvas::Canvas;
use crate::constants::*;
use crate::pathfinding::find_connected_sand;
use crate::pathfinding::find_spanning_group;
use derivative::Derivative;
use enum_map::EnumMap;
use image::Rgba;
use imageproc::drawing;
use imageproc::rect::Rect;
use nanorand::RandomGen;
use nanorand::Rng;
use nanorand::WyRand;
use ndarray::s;
use ndarray::Array;
use ndarray::Array2;
use piston_window::graphics;
use piston_window::prelude::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Game {
    rng: WyRand,
    #[derivative(Debug = "ignore")]
    glyph_cache: Glyphs,
    canvas: Canvas,
    sand: Array2<Option<Color>>,
    animation: Option<(f64, Animation)>,
    paused: bool,
    elapsed_time: f64,
    next_move: f64,
    next_physics_update: f64,
    control_updates: EnumMap<Direction, Option<f64>>,
    queue_drop: bool,
    falling_block_pos: Option<Block>,
}

impl Game {
    pub fn new(window: &mut PistonWindow) -> Self {
        Self {
            rng: WyRand::new(),
            glyph_cache: Glyphs::from_bytes(
                PIXEL_FONT,
                window.create_texture_context(),
                TextureSettings::new(),
            )
            .unwrap(),
            canvas: Canvas::new(window),
            sand: Array2::default([
                window.size().width as usize / SAND_SIZE,
                window.size().height as usize / SAND_SIZE,
            ]),
            animation: None,
            paused: false,
            elapsed_time: 0.0,
            next_move: MOVE_DELAY,
            next_physics_update: 0.0,
            control_updates: Default::default(),
            queue_drop: false,
            falling_block_pos: None,
        }
    }

    fn reset(&mut self) {
        self.sand.assign(&Array::from_elem(1, None));
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
                    Key::Space => {
                        self.queue_drop = true;
                    }
                    Key::P => {
                        self.paused = !self.paused;
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
                    if let Some(block) = self
                        .falling_block_pos
                        .filter(|_| self.can_move(Direction::Left))
                    {
                        self.falling_block_pos = Some(block.dec_x());
                    }
                }
                Direction::Right => {
                    if let Some(block) = self
                        .falling_block_pos
                        .filter(|_| self.can_move(Direction::Right))
                    {
                        self.falling_block_pos = Some(block.inc_x());
                    }
                }
                Direction::Down => {
                    if let Some(block) = self.falling_block_pos {
                        if self.can_move(Direction::Down) {
                            self.falling_block_pos = Some(block.inc_y())
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
        if self.paused {
            return;
        }

        if self.run_animation(event.dt) {
            // If we are in the middle of an animation, let run_animation() handle it, the game is
            // effectively frozen
            return;
        } else if let Some((_, animation)) = self.animation.take() {
            // The animation is complete, do whatever needs to be done now that the animation's
            // finished
            match animation {
                Animation::RemoveLine {
                    affected_pixels, ..
                } => {
                    for (px, py) in affected_pixels {
                        self.sand[[px, py]] = None;
                    }
                }
            }
        }

        self.elapsed_time += event.dt;

        self.control_updates = self.control_updates.map(|input, update| {
            if let Some(update) = update.filter(|update| self.elapsed_time >= *update) {
                self.move_block(input);
                Some(update + INPUT_DELAY)
            } else {
                update
            }
        });

        if self.queue_drop {
            while self.falling_block_pos.is_some() {
                self.move_block(Direction::Down);
            }
            self.queue_drop = false;
        }

        if self.elapsed_time >= self.next_physics_update {
            self.run_sand_physics();
            self.next_physics_update += PHYSICS_DELAY;
        }

        if let Some((x, y)) = find_spanning_group(&self.sand) {
            self.animation = Some((
                0.0,
                Animation::RemoveLine {
                    flash_state: false,
                    affected_pixels: find_connected_sand(&self.sand, x, y),
                },
            ));
            // for (px, py) in find_connected_sand(&self.sand, x, y) {
            //     self.sand[[px, py]] = None;
            // }
        }

        if self.elapsed_time >= self.next_move {
            if self.falling_block_pos.is_some() {
                if self.control_updates[Direction::Down].is_none() {
                    self.move_block(Direction::Down);
                }
            } else {
                self.falling_block_pos = Some(
                    self.rng
                        .generate::<Block>()
                        .with_pos(self.sand.dim().0 / 2 - 1, 0),
                );
            }
            self.next_move += MOVE_DELAY;
        }
    }

    fn run_animation(&mut self, delta: f64) -> bool {
        let Some((animation_ts, animation)) = &mut self.animation else {
            return false;
        };

        *animation_ts += delta;

        match animation {
            Animation::RemoveLine { flash_state, .. } => {
                *flash_state = if (..FLASH_DELAY).contains(animation_ts)
                    || (FLASH_DELAY * 2.0..FLASH_DELAY * 3.0).contains(animation_ts)
                {
                    false
                } else if *animation_ts <= FLASH_DELAY * 4.0 {
                    true
                } else {
                    return false;
                };
            }
        };

        return true;
    }

    fn can_move(&self, direction: Direction) -> bool {
        if let Some(block) = self.falling_block_pos {
            match direction {
                Direction::Left => {
                    // TODO: Check sand
                    block.x > 0
                        && block.coords().all(|(px, py)| {
                            self.sand
                                .slice(s![px - 1, py..py + SAND_BLOCK_SIZE])
                                .iter()
                                .all(Option::is_none)
                        })
                }
                Direction::Right => {
                    // TODO: Check sand
                    block.x < self.sand.dim().0 - (SAND_BLOCK_SIZE * block.width())
                        && block.coords().all(|(px, py)| {
                            self.sand
                                .slice(s![px + SAND_BLOCK_SIZE, py..py + SAND_BLOCK_SIZE])
                                .iter()
                                .all(Option::is_none)
                        })
                }
                Direction::Down => {
                    block.y < self.sand.dim().1 - (SAND_BLOCK_SIZE * block.height())
                        && block.coords().all(|(px, py)| {
                            self.sand
                                .slice(s![px..px + SAND_BLOCK_SIZE, py + SAND_BLOCK_SIZE])
                                .iter()
                                .all(Option::is_none)
                        })
                }
            }
        } else {
            false
        }
    }

    fn add_sand_block(&mut self) {
        if let Some(block) = self.falling_block_pos {
            for (px, py) in block.coords() {
                self.sand
                    .slice_mut(s![px..px + SAND_BLOCK_SIZE, py..py + SAND_BLOCK_SIZE])
                    .assign(&Array::from_elem(1, Some(block.color)));
            }
        }
    }

    fn run_sand_physics(&mut self) {
        let last_sand = self.sand.clone();
        // The bottom line will not move so we can skip it, and not worry about the bottom edge
        // case
        for y in (0..self.sand.dim().1 - 1).rev() {
            for x in 0..self.sand.dim().0 {
                if self.sand[[x, y]].is_none() {
                    continue;
                }

                // Check directly below
                if self.sand[[x, y + 1]].is_none() {
                    self.sand[[x, y + 1]] = self.sand[[x, y]];
                    self.sand[[x, y]] = None;
                    continue;
                }
            }

            for x in 0..self.sand.dim().0 {
                if self.sand[[x, y]].is_none() {
                    continue;
                }

                // TODO: Make which direction the sand actually goes randomly decided

                // Check bottom left
                if x > 0 && self.sand[[x - 1, y + 1]].is_none() {
                    self.sand[[x - 1, y + 1]] = self.sand[[x, y]];
                    self.sand[[x, y]] = None;
                    continue;
                }

                // Check bottom right
                if x < self.sand.dim().0 - 1 && self.sand[[x + 1, y + 1]].is_none() {
                    self.sand[[x + 1, y + 1]] = self.sand[[x, y]];
                    self.sand[[x, y]] = None;
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
        for ((x, y), color) in self
            .sand
            .indexed_iter()
            .filter_map(|(pos, pixel)| pixel.map(|p| (pos, p)))
        {
            // TODO: Put this into the filter expression, maybe?
            if let Some((
                _,
                Animation::RemoveLine {
                    flash_state: false,
                    affected_pixels,
                },
            )) = &self.animation
            {
                if affected_pixels.contains(&(x, y)) {
                    continue;
                }
            }

            // buffer.put_pixel(x as u32, y as u32, Rgba([0, 255, 0, 255]));
            drawing::draw_filled_rect_mut(
                buffer,
                Rect::at((x * SAND_SIZE) as i32, (y * SAND_SIZE) as i32)
                    .of_size(SAND_SIZE as u32, SAND_SIZE as u32),
                color.pixel_color(),
            );
        }
        self.canvas.render(context, g);

        // for ((x, y), _) in self.blocks.indexed_iter().filter(|(_, x)| **x) {
        //     self.draw_block(x, y, [1.0, 0.0, 0.0, 1.0], context, g);
        // }

        if let Some(block) = self.falling_block_pos {
            block.render(context, g);
        }

        // Render paused text
        if self.paused {
            piston_window::Text::new(32).draw_pos("PAUSED", [32.0, 64.0], &mut self.glyph_cache, &Default::default(), context.transform, g).unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    x: usize,
    y: usize,
    shape: Shape,
    color: Color,
}

impl Block {
    fn with_pos(mut self, x: usize, y: usize) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    fn inc_x(mut self) -> Self {
        self.x += 1;
        self
    }

    fn dec_x(mut self) -> Self {
        self.x -= 1;
        self
    }

    fn inc_y(mut self) -> Self {
        self.y += 1;
        self
    }

    pub fn coords(&self) -> impl Iterator<Item = (usize, usize)> {
        self.shape.coords(self.x, self.y)
    }

    fn width(&self) -> usize {
        self.shape.shape().dim().0
    }

    fn height(&self) -> usize {
        self.shape.shape().dim().1
    }

    fn render(&self, context: graphics::Context, g: &mut G2d) {
        for (px, py) in self.coords() {
            let (x, y) = ((px * SAND_SIZE) as f64, (py * SAND_SIZE) as f64);
            graphics::rectangle_from_to(
                self.color.float_color(),
                [x, y],
                [x + BLOCK_SIZE as f64, y + BLOCK_SIZE as f64],
                context.transform,
                g,
            );
        }
    }
}

impl<Generator: Rng<OUTPUT>, const OUTPUT: usize> RandomGen<Generator, OUTPUT> for Block {
    fn random(rng: &mut Generator) -> Self {
        Self {
            x: 0,
            y: 0,
            shape: rng.generate(),
            color: rng.generate(),
        }
    }
}

#[derive(Debug, Clone)]
enum Animation {
    RemoveLine {
        flash_state: bool,
        affected_pixels: Vec<(usize, usize)>,
    },
}

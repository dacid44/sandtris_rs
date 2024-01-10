use crate::canvas::Canvas;
use crate::constants::*;
use crate::pathfinding::find_connected_sand;
use crate::pathfinding::find_spanning_group;
use crate::physics::run_rng_physics;
use derivative::Derivative;
use enum_map::EnumMap;
use graphics::ImageSize;
use graphics::Transformed;
use image::GenericImage;
use image::GenericImageView;
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
    text_textures: TextTextures,
    canvas: Canvas,
    sand: Array2<Option<Color>>,
    animation: Option<(f64, Animation)>,
    play_mode: PlayMode,
    elapsed_time: f64,
    next_move: f64,
    next_physics_update: f64,
    control_updates: EnumMap<Direction, Option<f64>>,
    queue_drop: bool,
    falling_block: Option<Block>,
    next_block: Block,
    score: usize,
    combo: usize,
}

impl Game {
    pub fn new(window: &mut PistonWindow) -> Self {
        let mut rng = WyRand::new();
        let next_block = rng.generate();
        Self {
            rng,
            text_textures: TextTextures::new(window),
            canvas: Canvas::new(window),
            sand: Array2::default([BOARD_SIZE.0 / SAND_SIZE, BOARD_SIZE.1 / SAND_SIZE]),
            animation: None,
            play_mode: PlayMode::Playing,
            elapsed_time: 0.0,
            next_move: MOVE_DELAY,
            next_physics_update: 0.0,
            control_updates: Default::default(),
            queue_drop: false,
            falling_block: None,
            next_block,
            score: 0,
            combo: 0,
        }
    }

    fn reset(&mut self) {
        self.sand.assign(&Array::from_elem(1, None));
        self.animation = None;
        self.play_mode = PlayMode::Playing;
        self.next_move = self.elapsed_time + MOVE_DELAY;
        self.next_physics_update = self.elapsed_time;
        self.queue_drop = false;
        self.falling_block = None;
        self.next_block = self.rng.generate();
        self.score = 0;
        self.combo = 1;
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Some(button) = event.press_args() {
            match button {
                Button::Keyboard(key) => match key {
                    Key::Left => {
                        if self.play_mode == PlayMode::Playing {
                            self.move_block(Direction::Left);
                        }
                        self.control_updates[Direction::Left] =
                            Some(self.elapsed_time + FIRST_INPUT_DELAY);
                    }
                    Key::Right => {
                        if self.play_mode == PlayMode::Playing {
                            self.move_block(Direction::Right);
                        }
                        self.control_updates[Direction::Right] =
                            Some(self.elapsed_time + FIRST_INPUT_DELAY);
                    }
                    Key::Down => {
                        if self.play_mode == PlayMode::Playing {
                            self.move_block(Direction::Down);
                        }
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
                        self.play_mode = self.play_mode.toggle_pause();
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
                        .falling_block
                        .filter(|_| self.can_move(Direction::Left))
                    {
                        self.falling_block = Some(block.dec_x());
                    }
                }
                Direction::Right => {
                    if let Some(block) = self
                        .falling_block
                        .filter(|_| self.can_move(Direction::Right))
                    {
                        self.falling_block = Some(block.inc_x());
                    }
                }
                Direction::Down => {
                    if let Some(block) = self.falling_block {
                        if self.can_move(Direction::Down) {
                            self.falling_block = Some(block.inc_y())
                        } else {
                            self.add_sand_block();
                            self.falling_block = None;
                            self.combo = 0;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, event: &UpdateArgs) {
        if self.play_mode != PlayMode::Playing {
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
                    self.combo += 1;
                    self.score += affected_pixels.len() * self.combo;
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
            while self.falling_block.is_some() {
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
        }

        if self.elapsed_time >= self.next_move {
            if self.falling_block.is_some() {
                if self.control_updates[Direction::Down].is_none() {
                    self.move_block(Direction::Down);
                }
            } else {
                self.falling_block = Some({
                    self.next_block.with_pos(
                        self.sand.dim().0 / 2 - self.next_block.width() * SAND_BLOCK_SIZE / 2,
                        0,
                    )
                });
                self.next_block = self.rng.generate();
                if !self.can_move(Direction::Down) {
                    self.play_mode = PlayMode::GameOver
                }
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
        if let Some(block) = self.falling_block {
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
        if let Some(block) = self.falling_block {
            for (px, py) in block.coords() {
                self.sand
                    .slice_mut(s![px..px + SAND_BLOCK_SIZE, py..py + SAND_BLOCK_SIZE])
                    .assign(&Array::from_elem(1, Some(block.color)));
            }
        }
    }

    fn run_sand_physics(&mut self) {
        // let last_sand = self.sand.clone();
        // The bottom line will not move so we can skip it, and not worry about the bottom edge
        // case
        // for y in (0..self.sand.dim().1 - 1).rev() {
        //     for x in 0..self.sand.dim().0 {
        //         if self.sand[[x, y]].is_none() {
        //             continue;
        //         }
        //
        //         // Check directly below
        //         if self.sand[[x, y + 1]].is_none() {
        //             self.sand[[x, y + 1]] = self.sand[[x, y]];
        //             self.sand[[x, y]] = None;
        //             continue;
        //         }
        //     }
        //
        //     for x in 0..self.sand.dim().0 {
        //         if self.sand[[x, y]].is_none() {
        //             continue;
        //         }
        //
        //         // TODO: Make which direction the sand actually goes randomly decided
        //
        //         // Check bottom left
        //         if x > 0 && self.sand[[x - 1, y + 1]].is_none() {
        //             self.sand[[x - 1, y + 1]] = self.sand[[x, y]];
        //             self.sand[[x, y]] = None;
        //             continue;
        //         }
        //
        //         // Check bottom right
        //         if x < self.sand.dim().0 - 1 && self.sand[[x + 1, y + 1]].is_none() {
        //             self.sand[[x + 1, y + 1]] = self.sand[[x, y]];
        //             self.sand[[x, y]] = None;
        //             continue;
        //         }
        //     }
        // }
        run_rng_physics(&mut self.rng, self.sand.view_mut());
    }

    fn center_texture(
        width: u32,
        height: u32,
        context: graphics::Context,
        texture: &G2dTexture,
    ) -> graphics::Context {
        Self::center_texture_x(width, context, texture)
            .trans(0.0, (height / 2 - texture.get_height() / 2) as f64)
    }

    fn center_texture_x(
        width: u32,
        context: graphics::Context,
        texture: &G2dTexture,
    ) -> graphics::Context {
        context.trans((width / 2 - texture.get_width() / 2) as f64, 0.0)
    }

    fn draw_dashboard(&mut self, context: graphics::Context, g: &mut G2d) {
        let ui_width = WINDOW_SIZE.0 - BOARD_SIZE.0 as u32;
        let ui_height = WINDOW_SIZE.1 as u32;

        let context = context.trans(BOARD_SIZE.0 as f64, 0.0);

        // Draw background
        graphics::rectangle_from_to(
            UI_BACKGROUND_COLOR,
            [0.0, 0.0],
            [ui_width as f64, ui_height as f64],
            context.transform,
            g,
        );

        // Draw score
        let score_texture = self
            .text_textures
            .texture_with_background(
                &format!("{:0width$}", self.score, width = SCORE_DIGITS),
                SCORE_SCALE,
                TEXT_COLOR,
                UI_ELEMENT_BG_COLOR,
            )
            .unwrap();

        let score_context =
            Self::center_texture_x(ui_width, context, score_texture).trans(0.0, SCORE_Y as f64);

        graphics::image(score_texture, score_context.transform, g);

        let score_label_texture = self
            .text_textures
            .texture_with_background("SCORE", SCORE_LABEL_SCALE, TEXT_COLOR, UI_ELEMENT_BG_COLOR)
            .unwrap();

        graphics::image(
            score_label_texture,
            score_context
                .trans(0.0, -(score_label_texture.get_height() as f64))
                .transform,
            g,
        );

        // Draw next block display
        let next_block_context = context.trans(
            ui_width as f64 / 2.0 - NEXT_BLOCK_DISPLAY_WIDTH / 2.0,
            NEXT_BLOCK_Y as f64,
        );

        let next_block_label_texture = self
            .text_textures
            .texture_with_background(
                "NEXT",
                NEXT_BLOCK_LABEL_SCALE,
                TEXT_COLOR,
                UI_ELEMENT_BG_COLOR,
            )
            .unwrap();
        graphics::image(
            next_block_label_texture,
            next_block_context
                .trans(0.0, -(next_block_label_texture.get_height() as f64))
                .transform,
            g,
        );

        graphics::rectangle_from_to(
            UI_ELEMENT_BG_COLOR_FLOAT,
            [0.0, 0.0],
            [NEXT_BLOCK_DISPLAY_WIDTH, NEXT_BLOCK_DISPLAY_HEIGHT],
            next_block_context.transform,
            g,
        );

        let shape_context = next_block_context.trans(
            NEXT_BLOCK_DISPLAY_WIDTH / 2.0 - (self.next_block.width() * BLOCK_SIZE) as f64 / 4.0,
            NEXT_BLOCK_DISPLAY_HEIGHT / 2.0 - (self.next_block.height() * BLOCK_SIZE) as f64 / 4.0,
        ).scale(0.5, 0.5);

        self.next_block.render_origin(shape_context, g);
    }

    pub fn render(&mut self, context: graphics::Context, g: &mut G2d) {
        self.canvas.clear(Rgba([255, 255, 255, 255]));
        let buffer = self.canvas.image();

        // graphics::clear(CLEAR_COLOR, g);
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

            drawing::draw_filled_rect_mut(
                buffer,
                Rect::at((x * SAND_SIZE) as i32, (y * SAND_SIZE) as i32)
                    .of_size(SAND_SIZE as u32, SAND_SIZE as u32),
                color.pixel_color(),
            );
        }
        self.canvas.render(context, g);

        if let Some(block) = self.falling_block {
            block.render(context, g);
        }

        self.draw_dashboard(context, g);

        // Render paused text
        if self.play_mode == PlayMode::Paused {
            let texture = self.text_textures.texture("PAUSED", 6, TEXT_COLOR).unwrap();
            graphics::image(
                texture,
                Self::center_texture(
                    (self.sand.dim().0 * SAND_SIZE) as u32,
                    (self.sand.dim().1 * SAND_SIZE) as u32,
                    context,
                    texture,
                )
                .transform,
                g,
            );
        }

        // Render game over text
        if self.play_mode == PlayMode::GameOver {
            let texture = self
                .text_textures
                .texture("GAME OVER", 6, TEXT_COLOR)
                .unwrap();
            graphics::image(
                texture,
                Self::center_texture(
                    (self.sand.dim().0 * SAND_SIZE) as u32,
                    (self.sand.dim().1 * SAND_SIZE) as u32,
                    context,
                    texture,
                )
                .trans(0.0, texture.get_height() as f64 / (-7.0 / 4.0))
                .transform,
                g,
            );
            let restart_texture = self
                .text_textures
                .texture("PRESS R TO RESTART", 3, TEXT_COLOR)
                .unwrap();
            graphics::image(
                restart_texture,
                Self::center_texture(
                    (self.sand.dim().0 * SAND_SIZE) as u32,
                    (self.sand.dim().1 * SAND_SIZE) as u32,
                    context,
                    restart_texture,
                )
                .trans(0.0, restart_texture.get_height() as f64 / (7.0 / 4.0))
                .transform,
                g,
            )
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

    fn render_origin(&self, context: graphics::Context, g: &mut G2d) {
        for (px, py) in self.shape.coords(0, 0) {
            let (x, y) = ((px * SAND_SIZE) as f64, (py * SAND_SIZE) as f64);
            graphics::rectangle_from_to(self.color.float_color(), [x, y], [x + BLOCK_SIZE as f64, y + BLOCK_SIZE as f64], context.transform, g);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayMode {
    Playing,
    Paused,
    GameOver,
}

impl PlayMode {
    fn toggle_pause(&self) -> Self {
        match self {
            Self::Playing => Self::Paused,
            Self::Paused => Self::Playing,
            Self::GameOver => Self::GameOver,
        }
    }
}

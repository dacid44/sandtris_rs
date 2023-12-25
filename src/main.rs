mod canvas;
mod game;
mod blocks;

use piston_window::prelude::*;

fn main() {
    println!("Hello, world!");

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("sandtris_rs", [384, 512])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut game = game::Game::new(&mut window);

    while let Some(e) = window.next() {
        game.handle_event(&e);
        e.update(|args| game.update(args));
        window.draw_2d(&e, |c, g, _| {
            game.render(c, g);
        });
    }
}

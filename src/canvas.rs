use derivative::Derivative;
use image::{Rgba, RgbaImage};
use piston_window::prelude::*;
use piston_window::graphics;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Canvas {
    #[derivative(Debug="ignore")]
    texture_context: G2dTextureContext,
    dims: (u32, u32),
    buffer: image::RgbaImage,
}

impl Canvas {
    pub fn new(window: &mut PistonWindow) -> Self {
        let dims = (window.size().width as u32, window.size().height as u32);
        Self {
            texture_context: window.create_texture_context(),
            dims,
            buffer: RgbaImage::new(dims.0, dims.1),
        }
    }

    pub fn clear(&mut self, color: Rgba<u8>) {
        self.buffer = RgbaImage::from_pixel(self.dims.0, self.dims.1, color);
    }

    pub fn image(&mut self) -> &mut RgbaImage {
        &mut self.buffer
    }

    pub fn texture(&mut self) -> G2dTexture {
        Texture::from_image(
            &mut self.texture_context,
            &self.buffer,
            &TextureSettings::new(),
        )
        .unwrap()
    }

    pub fn render(&mut self, context: graphics::Context, g: &mut G2d) {
        graphics::image(&self.texture(), context.transform, g);
    }
}

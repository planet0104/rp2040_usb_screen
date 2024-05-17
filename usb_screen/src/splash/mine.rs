use core::slice;
use crate::canvas::Canvas;

use super::vector_2d::Vector2D;

pub struct Mine {
    pub position: Vector2D,
    image: [u8; 288]
}

impl Mine {
    pub fn new(position: Vector2D, image: [u8; 288]) ->Mine {
        Mine {position, image}
    }

    pub fn render<'a>(&mut self, canvas: &mut Canvas<'a>){
        let img = unsafe { slice::from_raw_parts(self.image.as_ptr() as *mut u16, self.image.len()/2) };
        canvas.draw_image(self.position.x as usize, self.position.y as usize, img, 12);
    }
}
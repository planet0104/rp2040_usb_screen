use core::slice;
use super::canvas::Canvas;

use super::vector_2d::Vector2D;

pub struct Mine {
    pub position: Vector2D,
    image: [u8; 288]
}

impl Mine {
    pub fn new(position: Vector2D, mut image: [u8; 288]) ->Mine {
        // 图像是LE格式，转换成BE，因为ST7789默认是BE模式
        crate::rgb565::rgb565_le_to_be(&mut image);
        Mine {position, image}
    }

    pub fn render<'a>(&mut self, canvas: &mut Canvas){
        let img = unsafe { slice::from_raw_parts(self.image.as_ptr() as *mut u16, self.image.len()/2) };
        canvas.draw_image(self.position.x as usize, self.position.y as usize, img, 12);
    }
}
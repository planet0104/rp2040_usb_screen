use nusb::Interface;
use anyhow::Result;
use crate::{rgb565::Rgb565Pixel, usb_screen::{clear_screen, draw_rgb_image, SCREEN_HEIGHT, SCREEN_WIDTH}};

pub fn draw(interface: &Interface) -> Result<()>{
    let img = image::open("assets/rgb24.bmp")?.to_rgb8();
    clear_screen(Rgb565Pixel::from_rgb(0, 0, 255), interface)?;
    let center_x = SCREEN_WIDTH/2-img.width() as u16/2;
    let center_y = SCREEN_HEIGHT/2 - img.height() as u16/2;
    draw_rgb_image(center_x, center_y, &img, interface)?;
    Ok(())
}
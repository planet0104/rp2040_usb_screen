use std::time::Duration;

use chrono::Local;
use image::buffer::ConvertBuffer;
use offscreen_canvas::{Font, FontSettings, OffscreenCanvas, BLUE, WHITE};
use anyhow::{anyhow, Result};

pub fn draw(
    #[cfg(feature = "usb-serial")]
    port: &mut dyn serialport::SerialPort,
    #[cfg(feature = "usb-raw")]
    interface:&nusb::Interface,
    screen_width: u16, screen_height: u16) -> Result<()>{
    let font_bytes:&[u8] = include_bytes!("../assets/VonwaonBitmap-16px.ttf");
    let font = Font::from_bytes(font_bytes, FontSettings::default()).map_err(|err| anyhow!("{err}"))?;
    let img = image::open("assets/rgb24.bmp")?.to_rgba8();
    let mut canvas = OffscreenCanvas::new(screen_width as u32, screen_height as u32, font);

    let center_x = screen_width as i32/2;
    let center_y = screen_height as i32/2;

    loop{
        canvas.clear(BLUE);
        canvas.draw_image_at(&img, center_x-img.width() as i32/2, center_y-img.height() as i32/2, None, None);

        let date = Local::now().format("%Y/%m/%d %H:%M:%S").to_string();
        canvas.draw_text(&date, WHITE, 16., 5, 105);

        #[cfg(feature = "usb-serial")]
        crate::usb_screen::draw_rgb_image_serial(0, 0, &canvas.image_data().convert(), port)?;
        #[cfg(feature = "usb-raw")]
        crate::usb_screen::draw_rgb_image(0, 0, &canvas.image_data().convert(), interface)?;

        std::thread::sleep(Duration::from_secs(1));
    }
}
use image::Rgb;
use anyhow::Result;

pub fn draw(
    #[cfg(feature = "usb-serial")]
    port: &mut dyn serialport::SerialPort,
    #[cfg(feature = "usb-raw")]
    interface:&nusb::Interface,
    width: u16, height: u16) -> Result<()>{
    let img = image::open("assets/rgb24.bmp")?.to_rgb8();

    #[cfg(feature = "usb-serial")]
    crate::usb_screen::clear_screen_serial(Rgb([0, 0, 255]), port, width, height)?;
    
    println!("clear screen...");
    #[cfg(feature = "usb-raw")]
    crate::usb_screen::clear_screen(Rgb([0, 0, 255]), interface, width, height)?;

    let center_x = width/2-img.width() as u16/2;
    let center_y = height/2 - img.height() as u16/2;

    println!("draw image...");
    #[cfg(feature = "usb-serial")]
    crate::usb_screen::draw_rgb_image_serial(center_x, center_y, &img, port)?;
    #[cfg(feature = "usb-raw")]
    crate::usb_screen::draw_rgb_image(center_x, center_y, &img, interface)?;
    Ok(())
}
use std::io::Cursor;
use anyhow::Result;
use image::{buffer::ConvertBuffer, imageops::resize, RgbImage, RgbaImage};

pub fn draw(
    #[cfg(feature = "usb-serial")]
    port: &mut dyn serialport::SerialPort,
    #[cfg(feature = "usb-raw")]
    interface:&nusb::Interface,
    screen_width: u16,
    screen_height: u16,
) -> Result<()>{
   
    let file = Cursor::new(include_bytes!("../assets/tothesky.gif"));

    let mut gif_opts = gif::DecodeOptions::new();
    // Important:
    gif_opts.set_color_output(gif::ColorOutput::Indexed);
    
    let mut decoder = gif_opts.read_info(file)?;
    let mut screen = gif_dispose::Screen::new_decoder(&decoder);

    let mut frames = vec![];
    while let Some(frame) = decoder.read_next_frame()? {
        screen.blit_frame(&frame)?;
        let pixels = screen.pixels_rgba();
        let mut data = vec![];
        for pix in pixels{
            data.extend_from_slice(&[pix.r, pix.g, pix.b, pix.a]);
        }
        let img = RgbaImage::from_raw(screen.width() as u32, screen.height() as u32, data.to_vec()).unwrap();
        let rgb:RgbImage = img.convert();
        let rgb = resize(&rgb, screen_width as u32, screen_height as u32, image::imageops::FilterType::Triangle);
        frames.push(rgb);
    }

    loop{
        for frame in frames.iter(){
            #[cfg(feature = "usb-serial")]
            crate::usb_screen::draw_rgb_image_serial(0, 0, frame, port)?;
            #[cfg(feature = "usb-raw")]
            crate::usb_screen::draw_rgb_image(0, 0, frame, interface)?;
        }
    }
}


#[test]
pub fn resize_gif() -> anyhow::Result<()>{
    use gif::{Encoder, Frame, Repeat};
    use image::{imageops::resize, RgbaImage};
    use image::GenericImage;
    let file = std::fs::File::open("assets/image.gif")?;

    let mut gif_opts = gif::DecodeOptions::new();
    // Important:
    gif_opts.set_color_output(gif::ColorOutput::Indexed);
    
    let mut decoder = gif_opts.read_info(file)?;
    let mut screen = gif_dispose::Screen::new_decoder(&decoder);

    let mut image = std::fs::File::create("assets/image1.gif")?;
    let mut encoder = Encoder::new(&mut image, 160, 128, &[])?;
    encoder.set_repeat(Repeat::Infinite)?;

    let mut i = 0;
    while let Some(frame) = decoder.read_next_frame()? {
        screen.blit_frame(&frame)?;
        let pixels = screen.pixels_rgba();
        let mut data = vec![];
        for pix in pixels{
            data.extend_from_slice(&[pix.r, pix.g, pix.b, pix.a]);
        }
        let img = RgbaImage::from_raw(screen.width() as u32, screen.height() as u32, data.to_vec()).unwrap();
        let mut img = resize(&img, 227, 128, image::imageops::FilterType::Lanczos3);
        let mut img = img.sub_image(33, 0, 160, 128).to_image();
        let mut frame = Frame::from_rgba(img.width() as u16, img.height() as u16, img.as_mut());
        frame.delay = 6;
        if i % 2 == 0{
            encoder.write_frame(&frame)?;
        }
        i+= 1;
    }
    Ok(())
}
use anyhow::{Ok, Result};
mod rgb565;
mod usb_screen;
mod draw_bitmap;
mod clock;

fn main() -> Result<()> {

    let interface = usb_screen::open_usb_screen("USB Screen", "62985215")?.unwrap();

    // draw_bitmap::draw(&interface)?;
    clock::draw(&interface)?;

    Ok(())
}


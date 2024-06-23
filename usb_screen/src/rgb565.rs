/// A 16bit pixel that has 5 red bits, 6 green bits and  5 blue bits
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Rgb565Pixel(pub u16);

impl Rgb565Pixel {
    pub const R_MASK: u16 = 0b1111_1000_0000_0000;
    pub const G_MASK: u16 = 0b0000_0111_1110_0000;
    pub const B_MASK: u16 = 0b0000_0000_0001_1111;

    /// Return the red component as a u8.
    ///
    /// The bits are shifted so that the result is between 0 and 255
    pub fn red(self) -> u8 {
        ((self.0 & Self::R_MASK) >> 8) as u8
    }
    /// Return the green component as a u8.
    ///
    /// The bits are shifted so that the result is between 0 and 255
    pub fn green(self) -> u8 {
        ((self.0 & Self::G_MASK) >> 3) as u8
    }
    /// Return the blue component as a u8.
    ///
    /// The bits are shifted so that the result is between 0 and 255
    pub fn blue(self) -> u8 {
        ((self.0 & Self::B_MASK) << 3) as u8
    }
}

impl Rgb565Pixel{
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u16 & 0b11111000) << 8) | ((g as u16 & 0b11111100) << 3) | (b as u16 >> 3))
    }
}

pub struct Rgb565Image<'a>{
    pub pixels: &'a mut [u16],
    pub width: u16,
    pub height: u16,
}

impl <'a> Rgb565Image<'a>{
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgb565Pixel{
        Rgb565Pixel(self.pixels[y as usize * self.width as usize + x as usize])
    }
}

// LE格式转换成BE格式
pub fn rgb565_le_to_be(image:&mut [u8]){
    for pix in image.chunks_mut(2){
        let color = u16::from_le_bytes([pix[0], pix[1]]);
        let bytes = color.to_be_bytes();
        pix[0] = bytes[0];
        pix[1] = bytes[1];
    }
}
// LE格式转换成BE格式
pub fn rgb565_le_to_be(image:&mut [u8]){
    for pix in image.chunks_mut(2){
        let color = u16::from_le_bytes([pix[0], pix[1]]);
        let bytes = color.to_be_bytes();
        pix[0] = bytes[0];
        pix[1] = bytes[1];
    }
}

#[inline]
pub fn rgb_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0b11111000) << 8) | ((g as u16 & 0b11111100) << 3) | (b as u16 >> 3)
}
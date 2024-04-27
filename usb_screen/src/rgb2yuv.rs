//https://segmentfault.com/a/1190000016443536

// 优化：1、使用查表
// 优化：2、使用u16

use crate::rgb565::Rgb565Pixel;

fn clamp_u8(x:i32) -> u8 {
    if x > 255 {
        return 255;
    } else if x < 0 {
        return 0;
    } else {
        return x as  u8;
    }
}

pub fn rgb_to_yuv420p(destination: &mut [u8], rgb:&[u8], width: usize, height: usize)
{
    let image_size = width * height;
    let mut upos = image_size;
    let mut vpos = upos + upos / 4;
    let mut  i = 0;

    for line in 0..height
    {
        if line % 2 == 0
        {
            for _ in (0..width).step_by(2)
            {
                let mut r = rgb[3 * i] as i32;
                let mut g = rgb[3 * i + 1] as i32;
                let mut b = rgb[3 * i + 2] as i32;
                let mut yt =  ((66*r + 129*g + 25*b + 128) >> 8) + 16;
                let ut =  (((-38*r) + (-74*g) + 112*b + 128) >> 8) + 128;
                let vt =  ((112*r + (-94*g) + (-18*b) + 128) >> 8) + 128;

                destination[i] = clamp_u8(yt);
                i += 1;
                destination[upos] = clamp_u8(ut);
                upos += 1;
                destination[vpos] = clamp_u8(vt);
                vpos += 1;

                r = rgb[3 * i] as i32;
                g = rgb[3 * i + 1] as i32;
                b = rgb[3 * i + 2] as i32;
                yt =  ((66*r + 129*g + 25*b + 128) >> 8) + 16;

                destination[i] = clamp_u8(yt);
                i += 1;
            }
        }
        else
        {
            for _ in 0..width
            {
                let r = rgb[3 * i] as i32;
                let g = rgb[3 * i + 1] as i32;
                let b = rgb[3 * i + 2] as i32;
                let yt =  ((66*r + 129*g + 25*b + 128) >> 8) + 16;

                destination[i] = clamp_u8(yt);
                i += 1;
            }
        }
    }
}

pub fn yuv_420p_to_rgb(destination: &mut[u8], yuv:&[u8], width: usize, height: usize)
{
    let image_size = width * height;
    let mut i = 0;

    for line in 0..height
    {
        for col in 0..width
        {
            let y = yuv[line*width+col] as i32;
            let u = yuv[(line/2)*(width/2)+(col/2)+image_size] as i32;
            let v = yuv[(line/2)*(width/2)+(col/2)+image_size+(image_size/4)] as i32;

            let c = y-16;
            let d = u-128;
            let e = v-128;

            let rt =  (298*c+408*e+128)>>8;
            let gt =  (298*c-100*d-208*e+128)>>8;
            let bt =  (298*c+516*d+128)>>8;

            destination[i] = clamp_u8(rt);
            i+= 1;
            destination[i] = clamp_u8(gt);
            i+= 1;
            destination[i] = clamp_u8(bt);
            i+= 1;
        }
    }
}

pub fn yuv_420p_to_rgb565(destination: &mut[u8], yuv:&[u8], width: usize, height: usize)
{
    let image_size = width * height;
    // let mut i = 0;

    for line in 0..height
    {
        for col in 0..width
        {
            let y = yuv[line*width+col] as i32;
            let u = yuv[(line/2)*(width/2)+(col/2)+image_size] as i32;
            let v = yuv[(line/2)*(width/2)+(col/2)+image_size+(image_size/4)] as i32;

            let c = y-16;
            let d = u-128;
            let e = v-128;

            let rt =  clamp_u8((298*c+408*e+128)>>8);
            let gt =  clamp_u8((298*c-100*d-208*e+128)>>8);
            let bt =  clamp_u8((298*c+516*d+128)>>8);

            let rgb565_pixel: Rgb565Pixel = Rgb565Pixel::from_rgb(rt, gt, bt);
            let be_bytes = rgb565_pixel.0.to_le_bytes();
            let i = line * width + col;
            destination[i*2] = be_bytes[0];
            destination[i*2+1] = be_bytes[1];
        }
    }
}
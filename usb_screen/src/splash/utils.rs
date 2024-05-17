use embassy_rp::clocks::RoscRng;
use rand_core::RngCore;
use micromath::F32Ext;

//返回[l, b) 区间的数
pub fn random_usize(rng: &mut RoscRng, l: usize, b: usize) -> usize{
    (random_float(rng)*(b as f32 - l as f32 + 1.0)).floor() as usize + l
}

pub fn random_float(rng: &mut RoscRng) -> f32{
    rng.next_u32() as f32 / u32::MAX as f32
}

//返回-1 <n <1范围内的随机浮点数
pub fn random_clamped(rng: &mut RoscRng) -> f32{ random_float(rng) - random_float(rng) }

pub fn clamp(arg: &mut f32, min: f32, max: f32){
    if *arg < min {
        *arg = min;
    }
    if *arg > max {
        *arg = max;
    }
}
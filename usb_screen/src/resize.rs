pub fn nearest_neighbor_interpolation(input: &[u16], width: usize, height: usize, new_width: usize, new_height:usize, output:&mut [u16]){
    let scale_x = new_width as f32 / width as f32;
    let scale_y = new_height as f32 / height as f32;

    for y in 0..new_height {
        for x in 0..new_width {
            // 计算原图中最近的像素位置
            let src_x = (x as f32 / scale_x) as usize;
            let src_y = (y as f32 / scale_y) as usize;

            // 从原图获取最近像素的颜色并设置到新图中
            let src_index = src_y * width + src_x;
            let dst_index = y * new_width + x;
            output[dst_index] = input[src_index];
        }
    }
}

pub fn magnify_rgb565(input: &[u16], width: usize, height: usize, output: &mut[u16]){
    let new_width = width * 2;

    for y in 0..height {
        for x in 0..width {
            // 原始像素索引
            let src_index = y * width + x;

            // 目标像素的索引范围（2x2）
            let dst_start_index = (y * 2) * new_width + (x * 2);
            output[dst_start_index] = input[src_index];
            output[dst_start_index + 1] = input[src_index];
            output[dst_start_index + new_width] = input[src_index];
            output[dst_start_index + new_width + 1] = input[src_index];
        }
    }
}
use core::slice;

use alloc::vec::Vec;
use byte_slice_cast::AsMutByteSlice;
use embassy_rp::clocks::RoscRng;
use micromath::F32Ext;
use super::canvas::Canvas;

use super::mine::Mine;
use super::neural_net::NeuralNet;
use super::vector_2d::Vector2D;
use super::utils::{ random_float,clamp };
use super::params::{ MAX_TURN_RATE, NUM_OUTPUTS, WINDOW_HEIGHT, WINDOW_WIDTH };

pub struct MineSweeper {
    tick: u64,
    //扫雷机的神经网络
    its_brain: NeuralNet,
    //扫雷机在世界坐标里的位置
    position: Vector2D,
    //扫雷机面对的方向
    look_at: Vector2D,
    //扫雷机的旋转
    rotation: f32,
    speed: f32,
    //以存储来自ANN的输出
    left_track: f32,
    right_track: f32,
    //最近的地雷的位置
    closest_mine: usize,
    current_img: usize,
    img_indexes: [usize; 5],
    images: Vec<Vec<Vec<u16>>>
}

impl MineSweeper {
    pub fn new(rng: &mut RoscRng) ->MineSweeper {
        //p0
        let mut p0 = Vec::new();
        p0.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p0.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        //p1
        let mut p1 = Vec::new();
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p10.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p11.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p12.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p13.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p14.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p15.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p16.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p17.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p18.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p1.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p19.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        //p2
        let mut p2 = Vec::new();
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p20.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p21.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p22.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p23.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p24.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p25.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p26.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p27.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p28.raw").as_ptr() as *mut u16, 16*16) }.to_vec());
        p2.push(unsafe { slice::from_raw_parts_mut(include_bytes!("../../assets/p29.raw").as_ptr() as *mut u16, 16*16) }.to_vec());

        // 图像是LE格式，转换成BE，因为ST7789默认是BE模式
        for img in &mut p0{
            crate::rgb565::rgb565_le_to_be(img.as_mut_byte_slice());
        }
        for img in &mut p1{
            crate::rgb565::rgb565_le_to_be(img.as_mut_byte_slice());
        }
        for img in &mut p2{
            crate::rgb565::rgb565_le_to_be(img.as_mut_byte_slice());
        }

        MineSweeper {
            tick: 0,
            rotation: 0.,
            left_track: 0.16,
            right_track: 0.16,
            closest_mine: 0,
            position: Vector2D::new(random_float(rng)*WINDOW_WIDTH as f32, random_float(rng)*WINDOW_HEIGHT as f32),
            speed: 0.0,
            look_at: Vector2D::new(0.0, 0.0),
            its_brain: NeuralNet::new(rng),
            current_img: 0,
            img_indexes: [0, 1, 2, 1, 0],
            images: alloc::vec![p0, p1, p2]
        }
    }

    pub fn put_weights(&mut self, w: &Vec<f32>){
        self.its_brain.put_weights(w)
    }

    pub fn render<'a>(&mut self, canvas:&mut Canvas){
        let skip = 6;
        if self.tick % skip == 0{
            self.current_img += 1;
            if self.current_img == self.img_indexes.len(){
                self.current_img = 0;
            }
        }
        let arr_idx = self.img_indexes[self.current_img];
        if arr_idx == 0{
            let img = &self.images[self.img_indexes[self.current_img]][0];
            canvas.draw_image(self.position.x as usize, self.position.y as usize, img, 16);
            return;
        }

        //rotation是不断增加的弧度
        let imgs = &self.images[arr_idx];
        let all_imgs: &[&Vec<u16>; 21] = &[&imgs[0],&imgs[1],&imgs[2],&imgs[3],&imgs[4],&imgs[5],&imgs[6],&imgs[7],&imgs[8],&imgs[9],&imgs[0],&imgs[9],&imgs[8],&imgs[7],&imgs[6],&imgs[5],&imgs[4],&imgs[3],&imgs[2],&imgs[1],&imgs[0]];
        let step = 6.29 / all_imgs.len() as f32;
        // 大概减去四分之一，90度。sweeper的角度是0度朝下
        let mut rotation: f32 = self.rotation - 1.56;
        while rotation < 0.{
            rotation += 6.29;
        }
        //0~360度对应0~6.29
        let rotation = rotation - ((rotation *100.) as i32 /629) as f32 * 6.29;
        let mut idx = (rotation / step) as usize;

        if idx >= all_imgs.len(){
            idx = all_imgs.len()-1;
        }

        if idx == 10{
            canvas.draw_image_flip_x(self.position.x as usize, self.position.y as usize, &all_imgs[idx], 16);
        }else if idx > 10{
            canvas.draw_image_flip_y(self.position.x as usize, self.position.y as usize, &all_imgs[idx], 16);
        }else{
            canvas.draw_image(self.position.x as usize, self.position.y as usize, &all_imgs[idx], 16);
        }
    }

    pub fn update(&mut self, mines: &Vec<Mine>) -> bool{
        self.tick += 1;
        //这一向量用来存放神经网络所有的输入
        let mut inputs = Vec::new();
        //计算从扫雷机到与其最近的地雷（两个点）之间的向量
        let mut closest_mine = self.get_closest_mine(mines);
        //将该向量规范化(扫雷机的视线向量不需要再做规范化，因为它的长度已经等于1了)
        Vector2D::normalize(&mut closest_mine);

        //计算向量和最近的矿井矢量的点积。 这将给我们的角度，我们需要转向面对最近的矿井
        let dot = Vector2D::dot(&self.look_at, &closest_mine);
        let sign = Vector2D::sign(&self.look_at, &closest_mine);

        let _ = inputs.push(dot*sign as f32);

        //再加入一个向量，扫雷机距离中心点的向量
        let mut center_pos = Vector2D::sub(&self.position, &Vector2D::new(WINDOW_WIDTH as f32/2., WINDOW_HEIGHT as f32/2.));
        Vector2D::normalize(&mut center_pos);
        let dot = Vector2D::dot(&self.look_at, &center_pos);
        let sign = Vector2D::sign(&self.look_at, &center_pos);
        let _ = inputs.push(dot*sign as f32);

        //更新大脑，并从网络得到输出
        let output = self.its_brain.update(&mut inputs);
        
        //确保在计算输出时没有错误
        if output.len() < NUM_OUTPUTS as usize {
            return false;
        }
        //把输出复制到扫雷机的左、右履带轮轨
        self.left_track = output[0]/2.;
        self.right_track = output[1]/2.;

        //计算驾驶的力
        //扫雷机的转动力是利用施加到它左、右履带轮轨上的力之差来计算的。
        //并规定，施加到左轨道上的力减去右轨道上的力，就得到扫雷机车辆的转动力。
        let mut rot_force = self.left_track - self.right_track;

        //进行左转或右转
        clamp(&mut rot_force, -MAX_TURN_RATE, MAX_TURN_RATE);
        self.rotation += rot_force;

        if self.rotation > 6.29{
            self.rotation = 0.;
        }
        if self.rotation < -6.29{
            self.rotation = 0.;
        }

        //扫雷机车的行进速度为它的左侧轮轨速度与它的右侧轮轨速度的和。
        self.speed = self.left_track + self.right_track;

        //更新视线角度
        self.look_at.x = - self.rotation.sin();
        self.look_at.y = self.rotation.cos();

        //更新位置
        self.position += Vector2D::mul(&self.look_at, self.speed);
        
        //屏幕越界处理
        if self.position.x > WINDOW_WIDTH as f32 { self.position.x = WINDOW_WIDTH as f32; }
        if self.position.x < 0.0 { self.position.x = 0.; }
        if self.position.y > WINDOW_HEIGHT as f32 { self.position.y = WINDOW_HEIGHT as f32; }
        if self.position.y < 0.0 { self.position.y = 0.0; }

        true
    }

    //检查扫雷机看它是否已经发现地雷
    //此函数检查与其最近的矿区的碰撞（先计算并存储在self.closest_mine中）
    pub fn check_for_mine(&self, mines: &Vec<Mine>, size: f32) -> i32 {
        let dist_to_object = Vector2D::sub(&self.position, &mines[self.closest_mine].position);
        //println!("dist_to_object.len() = {}", dist_to_object.len());
        if Vector2D::length(&dist_to_object) < (size+5.0) {
            return self.closest_mine as i32;
        }
        -1
    }

    //返回一个向量到最邻近的地雷
    pub fn get_closest_mine(&mut self, mines: &Vec<Mine>) ->Vector2D {
        let mut closest_so_far = 99999.0;
        let mut closest_object = Vector2D::new(0.0, 0.0);
        for i in 0..mines.len() {
            let len_to_object = Vector2D::length(&Vector2D::sub(&mines[i].position, &self.position));
            if len_to_object < closest_so_far {
                closest_so_far = len_to_object;
                closest_object = Vector2D::sub(&self.position, &mines[i].position);
                self.closest_mine = i;
            }
        }
        closest_object
    }
}
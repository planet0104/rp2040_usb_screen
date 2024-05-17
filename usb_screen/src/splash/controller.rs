use alloc::vec::Vec;
use embassy_rp::clocks::RoscRng;

use crate::canvas::Canvas;
use crate::rgb565::Rgb565Pixel;

use super::mine::Mine;
use super::params::{ MINE_SCALE, WINDOW_WIDTH, WINDOW_HEIGHT, NUM_MINES };
//控制器
use super::sweeper::MineSweeper;
use super::vector_2d::Vector2D;
use super::utils::{random_float, random_usize};

pub struct Controller{
    //扫雷机
    sweepers: [MineSweeper; 2],
    //地雷
    mines: Vec<Mine>,
    mine_images: [[u8; 288]; 4],
    rng: RoscRng,
}

impl Controller{
    pub fn new(mut rng: RoscRng) -> Controller {
        
        //让我们创建扫雷器
        let mut sweepers =  [MineSweeper::new(&mut rng), MineSweeper::new(&mut rng)];
       
        let net1 = [1.1761749, -2.3490496, 1.7504807, -1.6364365, -1.0930712, 0.55222297, 0.45217142, -0.41241857, 1.3328688, -1.264446, 0.76628166, -0.43053904, -1.3300905, 1.8379011, 0.84241253, -0.12235697, 0.42485982, -0.095809616, 2.3196793, 0.6394144, -1.2571108, -0.7584192, -1.2057884, -0.07373144, 0.77470946, 1.9586271, 0.27730122, 0.35081396, 1.2270349, -1.8249221, -1.0491095, -2.5858853, -0.953734, -0.87491363, 0.05362436, 0.7133873, 0.7683418, 0.3845099, 2.7733912, -0.87690324, -0.25711563, 1.0370685, 2.5344212, 1.1639981, -0.06830944, -0.9006881, -0.55249476, 0.7351557, -1.678612, 0.37971485, 0.17990036, -0.025974998, 1.7849637, 1.5416185, 1.084895, -2.5761595, -0.40175754, -0.70933163, -1.7338147, 1.0688963, 1.6158317, -0.30886626];
        let net2 = [0.13201556, 0.469124, -0.7255697, 1.8081125, -0.86148745, -0.13453819, 1.8926556, -0.05286616, 2.1018198, -0.16885431, 0.20330733, -0.4245945, -0.778438, -0.46245816, 1.2228462, -0.45225507, 0.9930343, 0.62506, -0.6748941, 1.6819415, 1.7669466, -0.50378263, 1.9861647, 2.6194286, -0.38949698, 0.13888855, 1.4205542, 0.68550396, 1.2150792, 0.91407335, -0.96463513, 0.28667438, -0.15604472, -0.22056536, -1.2475377, 1.042038, 0.51485443, 2.1506763, 0.9952272, 0.5168268, -1.9681755, -0.42918712, 0.15294977, -1.5091485, -2.1029205, 2.388837, -1.2438533, -0.26980677, -0.09587651, -0.21449852, 0.6029729, -0.65982354, 2.7028482, 1.1465942, 2.8347497, -1.4040165, 1.9681041, 0.019982278, 0.63877976, -0.62680686, 0.7078759, -0.6184324];
        let vec1 = Vec::from(net1);
        let vec2 = Vec::from(net2);
        sweepers[0].put_weights(&vec1);
        sweepers[1].put_weights(&vec2);
        
        let mut mines = Vec::new();

        let mine_images = [
         include_bytes!("../../assets/apple.raw").clone(),
         include_bytes!("../../assets/cherry.raw").clone(),
         include_bytes!("../../assets/strawberry.raw").clone(),
         include_bytes!("../../assets/tomato.raw").clone()];

        //在应用程序窗口内的随机位置初始化地雷
        for _ in 0..NUM_MINES {
            let _ = mines.push(Mine::new(Vector2D::new(
                random_float(&mut rng)*(WINDOW_WIDTH as f32 - 10.),
                random_float(&mut rng)*(WINDOW_HEIGHT as f32 - 10.)), mine_images[random_usize(&mut rng, 0, mine_images.len()-1)].clone()));
        }

        Controller {
            sweepers,
            mines,
            mine_images,
            rng
        }
    }

    pub fn update(&mut self) -> bool {
        for s in &mut self.sweepers {
            let _ = s.update(&self.mines);
            //看是否找到了一个地雷
            let grab_hit = s.check_for_mine(&self.mines, MINE_SCALE);
            if grab_hit >= 0 {
                //地雷被发现，so随机改变一下它的位置
                self.mines[grab_hit as usize] = Mine::new(Vector2D::new(
                    random_float(&mut self.rng)*WINDOW_WIDTH as f32,
                    random_float(&mut self.rng)*WINDOW_HEIGHT as f32), self.mine_images[random_usize(&mut self.rng, 0, self.mine_images.len()-1)].clone());
            }
        }
        true
    }

    pub fn render<'a>(&mut self, canvas: &mut Canvas<'a>) {
        //清空屏幕
        canvas.clear(Rgb565Pixel::from_rgb(0, 0, 0).0);
        //绘制地雷
        for mine in self.mines.iter_mut(){
            mine.render(canvas);
        }

        //render the sweepers
        for sweeper in self.sweepers.iter_mut(){
            sweeper.render(canvas);
        }
    }
}
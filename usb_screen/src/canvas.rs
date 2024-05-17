
pub struct Canvas<'a>{
    pub buf: &'a mut [u16],
    pub width: usize,
    pub height: usize,
}

impl <'a> Canvas<'a>{
    pub fn clear(&mut self, color: u16){
        self.buf.fill(color);
    }
    
    pub fn draw_image(&mut self, x: usize, y: usize, image:&[u16], image_width: usize){
        let mut begin = y*self.width + x;
        for row in image.chunks(image_width){
            let mut end = begin+image_width;
            if end > self.buf.len(){
                end = self.buf.len();                
            }
            let len = end as i32 - begin as i32;
            if len < 0{
                continue;
            }
            self.buf[begin..end].copy_from_slice(&row[0..len as usize]);
            begin += self.width;
            if begin > self.buf.len(){
                break;
            }
        }
    }

    pub fn draw_image_flip_y(&mut self, x: usize, y: usize, image:&[u16], image_width: usize){
        let mut begin = y*self.width + x;
        for row in image.chunks(image_width).rev(){
            let mut end = begin+image_width;
            if end > self.buf.len(){
                end = self.buf.len();                
            }
            let len = end as i32 - begin as i32;
            if len < 0{
                continue;
            }
            self.buf[begin..end].copy_from_slice(&row[0..len as usize]);
            begin += self.width;
            if begin > self.buf.len(){
                break;
            }
        }
    }

    pub fn draw_image_flip_x(&mut self, x: usize, y: usize, image:&[u16], image_width: usize){
        let mut begin = y*self.width + x;
        for row in image.chunks(image_width){
            let mut row = row.to_vec();
            row.reverse();
            let mut end = begin+image_width;
            if end > self.buf.len(){
                end = self.buf.len();                
            }
            let len = end as i32 - begin as i32;
            if len < 0{
                continue;
            }
            self.buf[begin..end].copy_from_slice(&row[0..len as usize]);
            begin += self.width;
            if begin > self.buf.len(){
                break;
            }
        }
    }
}
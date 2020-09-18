#![feature(try_trait)]
#![allow(dead_code)]
#![allow(unused)]
#![feature(once_cell)]

mod olc_engine;
use olc_engine::engine::*;
use rand::prelude::ThreadRng;
use std::rc::Rc;

struct MyGame{
    print_data: String,
    timer: f32,
    bmp: Decal,
    points: [Vf2d;4],
    selected: Option<i32>,
}


impl Olc for MyGame{
    fn on_engine_start(& mut self)
                       -> bool {
        //instantiate global data here
        let mut decal = Decal::create(Some(
            Sprite::load_from_file::<BMPLoader>
                (".\\tank.bmp")
        ));
        self.bmp = decal;
        self.points[3] = Vf2d::new(10.0, 10.0);
        self.points[0] = Vf2d::new(320.0, 10.0);
        self.points[1] = Vf2d::new(320.0, 320.0);
        self.points[2] = Vf2d::new(10.0, 320.0);
        true
    }

    fn on_engine_update(&mut self, elapsed_time: f32)  -> bool {
        //clear(Pixel::VERY_DARK_BLUE);

        if get_mouse(0).pressed {
            self.selected = None;
            let mut i = 0;
            for i in 0..self.points.len(){
                if (mouse_pos() - self.points[i]).mag() < 10.0{
                    self.selected = Some(i as i32);
                }
            }
        }

        if let Some(point) = self.selected {
            self.points[point as usize] = mouse_pos();
        }

        draw_warped_decal(&self.bmp, self.points.to_vec());

        if get_mouse(0).released{
            self.selected = None;
        }

        for i in 0..self.points.len(){
            fill_circle(self.points[i], 10, Pixel::YELLOW);
        }

        if get_key(Key::ESCAPE).pressed{
            return false;
        }

        true
    }

    fn on_engine_destroy(&mut self) -> bool {
        true
    }
}

fn main() {
    let mut a = MyGame{
        print_data: "Test".to_string(),
        timer: 0.0,
        bmp: Decal::new(),
        points: [Vf2d::new(0.0,0.0); 4],
        selected: None,
    };
    if a.construct("Test Game", 800, 600,
                   1, 1, false, false)
    {
        a.start();
    }
}

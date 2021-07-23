use super::{
    olc::OlcData,
    camera::Camera,
    decal::{Decal, DecalInstance, SmallD},
    geometry::{Triangle, UV, Mesh, Vertex},
    layer::{LayerDesc, LayerInfo, LayerType, Image, EMPTY_IMAGE, PipelineBundle},
    pixel::{Pixel, PixelMode},
    platform::{PLATFORM_DATA, Platform, Key},
    renderer::Renderer,
    sprite::{Sprite},
    util::{HWButton, Mouse, Vf2d, Vi2d, BMPLoader, ImageLoader, PNGLoader},
};
use std::sync::Arc;

pub struct OLCEngine<D: OlcData + 'static> {
    pub app_name: String,
    pub is_focused: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub pixels_w: u32,
    pub pixels_h: u32,
    pub fps: u32,
    pub renderer: Renderer,
    pub camera: Camera,
    pub game_data: Box<D>,
    pub inv_screen_size: Vf2d,
    pub draw_target: u32,
    pub full_screen: bool,
    pub vsync: bool,
    pub layers: Vec<LayerDesc<D>>,
    pub mouse_position: Vi2d,
    pub font_decal: Decal,
    pub depth_buffer: Vec<f64>,
    pub window: winit::window::Window,
}

impl<D: OlcData + 'static> OLCEngine<D> {
    pub fn init(
        &mut self,
        app_name: &str,
        screen_width: u32,
        screen_height: u32,
        pixel_width: u32,
        pixel_height: u32,
        full_screen: bool,
        vsync: bool,
    ) {
        let tex_size_w = screen_width / pixel_width;
        let tex_size_h = screen_height / pixel_height;
        let inv_screen_size = Vf2d::from(((1.0 / tex_size_w as f32), (1.0 / tex_size_h as f32)));

        self.app_name = String::from(app_name);
        self.is_focused = true;
        self.window_width = screen_width;
        self.window_height = screen_height;
        self.pixels_w = tex_size_w;
        self.pixels_h = tex_size_h;
        self.pixel_width = pixel_width;
        self.pixel_height = pixel_height;
        self.inv_screen_size = inv_screen_size;
        self.fps = 0;
        self.full_screen = full_screen;
        self.vsync = vsync;
        self.layers = vec![];
        self.draw_target = 0;
        self.mouse_position = Vi2d::new(0, 0);
        self.font_decal = Decal::empty();
    }

    pub fn is_focused(&self) -> bool {
        unsafe { PLATFORM_DATA.key_focus }
    }

    pub fn get_key(&self, k: Key) -> HWButton {
        unsafe {
            if let Some(button) = PLATFORM_DATA.key_map.as_mut().unwrap().get(&k) {
                *button
            } else {
                HWButton::new()
            }
        }
    }

    pub fn set_key(&self, k: Key, hw: HWButton) {
        unsafe {
            let key = PLATFORM_DATA
                .key_map
                .as_mut()
                .unwrap()
                .entry(k)
                .or_insert(hw);
            *key = hw;
        }
    }
    pub fn clear_keys(&self) {
        unsafe {
            for (key, value) in PLATFORM_DATA.key_map.as_mut().unwrap() {
                (*value).pressed = false;
                (*value).released = false;
            }
        }
    }

    pub fn get_mouse(&self, b: Mouse) -> HWButton {
        unsafe {
            match b {
                Mouse::Left => PLATFORM_DATA.mouse_map.as_ref().unwrap()[0],
                Mouse::Right => PLATFORM_DATA.mouse_map.as_ref().unwrap()[1],
                Mouse::Middle => PLATFORM_DATA.mouse_map.as_ref().unwrap()[2],
            }
        }
    }

    pub fn render_gl_tris(&mut self, triangles: &[Triangle], texture: u32) {
        Renderer::draw_triangles(triangles, texture);
    }

    pub fn set_perspective(&self, fov: f32, aspect: f32, near_clip: f32, far_clip: f32) {
        let f_H = ((fov / 360.0 * std::f32::consts::PI) * near_clip) as f64;
        let f_W = f_H * aspect as f64;
        //(GL.glFrustum)(-f_W, f_W, -f_H, f_H, near_clip as f64, far_clip as f64)
    }

    pub fn set_mouse(&self, i: usize, k: HWButton) {
        unsafe { PLATFORM_DATA.mouse_map.as_mut().unwrap()[i] = k }
    }

    pub fn set_mouse_pos(&self, x: f32, y: f32) {
        unsafe {
            let px = PLATFORM_DATA.pixel_size.unwrap_or_default();
            let mp: Vi2d = (
                x.floor() as i32 * px.x + PLATFORM_DATA.view_position.unwrap().x,
                y.floor() as i32 * px.y + PLATFORM_DATA.view_position.unwrap().y,
            )
                .into();
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.window
                    .set_cursor_position(winit::dpi::PhysicalPosition { x: mp.x, y: mp.y });
            }

            #[cfg(target_arch = "wasm32")]
            {}

            /*if winapi::um::winuser::SetCursorPos(mp.x, mp.y) == 0
            {
                panic!("SetCursorPos failed");
            }*/
            PLATFORM_DATA.raw_mouse_position = Some((mp.x as f32, mp.y as f32).into());
            PLATFORM_DATA.mouse_position = Some((x.floor(), y.floor()).into());
        }
    }

    #[cfg(feature = "web-sys")]
    pub fn set_web_mouse(&self) {}

    #[cfg(feature = "web-sys")]
    pub fn check_mouse_lock(&self) -> bool {
        //self.window.has_pointer_grab()
        false
    }

    #[cfg(target_arch = "wasm32")]
    pub fn request_mouse_lock(&self) {
        self.window
            .set_cursor_grab(true)
            .expect("Can't grab cursor");
    }

    pub fn clear_depth_buffer(&mut self) {
        self.depth_buffer = vec![0.0; (self.pixels_w * self.pixels_h) as usize];
    }

    pub fn update_depth_buffer(&mut self, i: usize, d: f64) {
        self.depth_buffer[i] = d;
    }

    pub fn check_depth_buffer(&mut self, i: usize) -> f64 {
        self.depth_buffer[i]
    }

    pub fn mouse_x(&self) -> f32 {
        unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().x }
    }

    pub fn mouse_y(&self) -> f32 {
        unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().y }
    }

    pub fn mouse_wheel(&self) -> i32 {
        0
    }

    //pub fn get_window_mouse() -> Vi2d { Vi2d }

    pub fn get_mouse_pos(&self) -> Vf2d {
        unsafe {
            let m = PLATFORM_DATA.mouse_position;
            m.unwrap()
        }
    }
    pub fn get_raw_mouse_pos(&self) -> Vf2d {
        unsafe {
            let m = PLATFORM_DATA.raw_mouse_position;
            m.unwrap()
        }
    }

    //Utility
    pub fn screen_width(&self) -> i32 {
        unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().x }
    }

    pub fn screen_height(&self) -> i32 {
        unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().y }
    }

    pub fn get_draw_target_width(&self) -> i32 {
        self.get_draw_target_ref().sprite.width as i32
    }

    pub fn get_draw_target_height(&self) -> i32 {
        self.get_draw_target_ref().sprite.height as i32
    }

    pub fn set_screen_size(&self, w: i32, h: i32) {}

    //This is overriden by drawing onto Layers.
    //We may want to do a "DrawTarget" Trait.
    //pub fn set_draw_target(target: &mut Sprite) {}
    //pub fn get_draw_target() -> Sprite { Sprite }

    pub fn get_fps(&self) -> i32 {
        0
    }

    pub fn get_elapsed_time(&self) -> f32 {
        0.0
    }

    pub fn get_window_size(&self) -> Vi2d {
        unsafe { PLATFORM_DATA.window_size.unwrap() }
    }
    pub fn get_window_position(&self) -> Vi2d {
        unsafe { PLATFORM_DATA.view_position.unwrap() }
    }

    pub fn get_screen_size_in_game_pixels(&self) -> Vi2d {
        Vi2d::new(self.window_width as i32, self.window_height as i32)
    }
    pub fn get_pixel_size(&self) -> Vi2d {
        Vi2d::new(self.pixel_width as i32, self.pixel_height as i32)
    }

    pub fn set_draw_target(&mut self, layer_id: u32) {
        self.draw_target = layer_id;
        self.set_layer_update(layer_id, true);
    }
    pub fn reset_draw_target(&mut self) {
        //set back to background layer
        self.draw_target = self.layers[0].id;
    }

    pub fn get_draw_target(&mut self) -> Result<&mut LayerDesc<D>, ()> {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == self.draw_target {
                return Ok(layer);
            }
        }
        Err(())
    }

    pub fn get_draw_target_ref(&self) -> &Image {
        if let LayerInfo::Image(image) = &self.get_layer_ref(self.draw_target).unwrap().layer_info {
            image
        } else {
            &EMPTY_IMAGE
        }
    }

    pub fn set_layer_visible(&mut self, layer_id: u32, b: bool) {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == layer_id {
                layer.shown = b;
            }
        }
    }

    pub fn set_layer_update(&mut self, layer_id: u32, b: bool) {
        let layer_iter = self.layers.iter_mut();
        for layer in layer_iter {
            if layer.id == layer_id {
                if let LayerInfo::Image(image_info) = &mut layer.layer_info {
                    image_info.update = b;
                }
            }
        }
    }

    pub fn set_layer_offset(&self, layer: u8, x: f32, y: f32) {}

    pub fn set_layer_scale(&self, layer: u8, x: f32, y: f32) {}

    pub fn set_layer_tint(&self, layer: u8, tint: Pixel) {}

    //We'll come back to this
    //pub fn set_layer_custom_render_function

    pub fn get_layer_ref(&self, layer_id: u32) -> Option<&LayerDesc<D>> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Some(layer);
            }
        }
        None
    }

    pub fn get_image_layer_ref(&self, layer_id: u32) -> Option<&Image> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                if let LayerInfo::Image(image_info) = &layer.layer_info {
                    return Some(image_info);
                }
            }
        }
        None
    }

    pub fn get_layer(&self, layer_id: u32) -> Result<&LayerDesc<D>, ()> {
        let layer_iter = self.layers.iter();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Ok(layer);
            }
        }
        Err(())
    }
    pub fn get_layer_mut(&mut self, layer_id: u32) -> Result<&mut LayerDesc<D>, ()> {
        let layers: &mut Vec<LayerDesc<D>> = self.layers.as_mut();
        let layer_iter = layers.iter_mut();
        for layer in layer_iter {
            if layer.id == layer_id {
                return Ok(layer);
            }
        }
        Err(())
    }

    pub fn setup_render_layer(
        &mut self,
        layer_id: u32,
        pipeline: Option<PipelineBundle<D>>,
    ) {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == layer_id) {
            if let LayerInfo::Render(render_info) = &mut layer.layer_info {
                if let Some(data) = pipeline {
                    render_info.pipeline_bundle = Some(data);
                } else {
                    layer.setup_default_pipeline_data(&self.renderer);
                }
            }
        }
    }

    pub fn get_y_up_direction(&self) -> f32{
        unsafe{
            PLATFORM_DATA.y_up_direction
        }
    }

    pub fn add_layer(&mut self, layer_type: LayerType) -> u32 {
        let lay_id = self.renderer.create_texture(self.pixels_w, self.pixels_h);
        let mut layer = LayerDesc::empty(layer_type);
        layer.id = lay_id as u32;
        self.layers.push(layer);
        self.renderer.active_decals.insert(lay_id as usize, lay_id as u32);
        self.renderer.update_layer_texture_groups();
        lay_id as u32
    }

    pub fn add_layer_with_info(&mut self, mut layer_info: LayerInfo<D>) -> u32 {
        let lay_id = self.renderer.create_texture(self.pixels_w, self.pixels_h);
        if let LayerInfo::Image(image_data) = &mut layer_info {
            image_data.sprite = Sprite::new(self.pixels_w, self.pixels_h);
        }
        let mut layer = LayerDesc::new(layer_info);
        layer.id = lay_id as u32;
        self.layers.push(layer);
        self.renderer.active_decals.insert(lay_id as usize, lay_id as u32);
        self.renderer.update_layer_texture_groups();
        lay_id as u32
    }

    pub fn load_sprite(&self, path: &str) -> Sprite {
        Sprite::load_from_file::<BMPLoader>(path).unwrap()
    }

    fn push_decal_instance(&mut self, di: DecalInstance) {
        self.get_draw_target()
            .expect("Can't get draw target")
            .vec_decal_instance
            .push(di);
    }

    pub fn set_pixel_mode(&mut self, m: PixelMode) {}

    pub fn get_pixel_mode(&self) -> PixelMode {
        PixelMode::Normal
    }

    pub fn set_pixel_blend(&mut self, blend: f32) {}

    //DRAW ROUTINES
    pub fn draw(&mut self, x: i32, y: i32, p: Pixel) {
        if let LayerInfo::Image(image_data) = &mut self
            .get_draw_target()
            .expect("Can't get draw target")
            .layer_info
        {
            image_data.sprite.set_pixel(x as u32, y as u32, p);
        }
    }
    //DRAW ROUTINES
    pub fn draw_subregion(&mut self, x: i32, y: i32, width: i32, height: i32, p: &[Pixel]) {
        if let LayerInfo::Image(image_data) = &mut self
            .get_draw_target()
            .expect("Can't get draw target")
            .layer_info
        {
            image_data
                .sprite
                .set_region(x as u32, y as u32, width as u32, height as u32, p);
        }
    }

    pub fn draw_line(&mut self, pos1: Vi2d, pos2: Vi2d, p: Pixel) {
        self.draw_line_xy(pos1.x, pos1.y, pos2.x, pos2.y, p);
    }

    pub fn draw_line_xy(&mut self, mut x1: i32, mut y1: i32, mut x2: i32, mut y2: i32, p: Pixel) {
        if x1 < 0
            || x2 < 0
            || x1 > self.pixels_w as i32
            || x2 > self.pixels_w as i32
            || y1 < 0
            || y2 < 0
            || y1 > self.pixels_h as i32
            || y2 > self.pixels_h as i32
        {
            return;
        }

        let (mut x, mut y, mut dx, mut dy, mut dx1, mut dy1, mut px, mut py, mut xe, mut ye, mut i) =
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        dx = x2 - x1;
        dy = y2 - y1;

        if dx == 0 {
            if y2 < y1 {
                std::mem::swap(&mut y1, &mut y2);
            }
            for y in y1..=y2 {
                self.draw(x1, y, p);
            }
            return;
        }

        if dy == 0 {
            if x2 < x1 {
                std::mem::swap(&mut x1, &mut x2);
            }
            for x in x1..=x2 {
                self.draw(x, y1, p);
            }
            return;
        }

        dx1 = dx.abs();
        dy1 = dy.abs();
        px = 2 * dy1 - dx1;
        py = 2 * dx1 - dy1;
        if dy1 <= dx1 {
            if dx >= 0 {
                x = x1;
                y = y1;
                xe = x2;
            } else {
                x = x2;
                y = y2;
                xe = x1;
            }

            self.draw(x, y, p);

            for i in x..xe {
                if px < 0 {
                    px += 2 * dy1;
                } else {
                    if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                        y += 1;
                    } else {
                        y -= 1;
                    }
                    px += 2 * (dy1 - dx1);
                }
                self.draw(x, y, p);
            }
        } else {
            if dy >= 0 {
                x = x1;
                y = y1;
                ye = y2;
            } else {
                x = x2;
                y = y2;
                ye = y1;
            }
            self.draw(x, y, p);

            for i in y..ye {
                if py <= 0 {
                    py += 2 * dx1;
                } else {
                    if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                        x += 1
                    } else {
                        x -= 1;
                    }
                    py += 2 * (dx1 - dy1);
                }
                self.draw(x, y, p);
            }
        }
    }

    pub fn draw_circle(&mut self, pos: Vi2d, r: i32, p: Pixel, mask: u32) {
        self.draw_circle_xy(pos.x, pos.y, r, p, mask);
    }

    pub fn draw_circle_xy(&mut self, x: i32, y: i32, r: i32, pixel: Pixel, mask: u32) {
        if r < 0
            || x < -r
            || y < -r
            || x - self.get_draw_target_ref().sprite.width as i32 > r
            || y - self.get_draw_target_ref().sprite.height as i32 > r
        {
            return;
        }
        if r > 0 {
            let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
            while y0 >= x0 {
                if mask & 0x01 == 0x01 {
                    self.draw(x + x0, y - y0, pixel)
                };
                if mask & 0x04 == 0x04 {
                    self.draw(x + y0, y + x0, pixel)
                };
                if mask & 0x10 == 0x10 {
                    self.draw(x - x0, y + y0, pixel)
                };
                if mask & 0x40 == 0x40 {
                    self.draw(x - y0, y - x0, pixel)
                };
                if x0 != 0 && x0 != y0 {
                    if mask & 0x02 == 0x02 {
                        self.draw(x + y0, y - x0, pixel)
                    };
                    if mask & 0x08 == 0x08 {
                        self.draw(x + x0, y + y0, pixel)
                    };
                    if mask & 0x20 == 0x20 {
                        self.draw(x - y0, y + x0, pixel)
                    };
                    if mask & 0x80 == 0x80 {
                        self.draw(x - x0, y - y0, pixel)
                    };
                }
                x0 += 1;
                if d < 0 {
                    d += 4 * x0 + 6;
                } else {
                    y0 -= 1;
                    d += 4 * (x0 - y0) + 10;
                }
            }
        } else {
            self.draw(x, y, pixel);
        }
    }

    pub fn fill_circle(&mut self, pos: Vf2d, r: i32, p: Pixel) {
        self.fill_circle_xy(pos.x as i32, pos.y as i32, r, p);
    }

    pub fn fill_circle_xy(&mut self, mut x: i32, mut y: i32, r: i32, pixel: Pixel) {
        if r < 0
            || x < -r
            || y < -r
            || x - self.get_draw_target_ref().sprite.width as i32 > r
            || y - self.get_draw_target_ref().sprite.height as i32 > r
        {
            return;
        }

        if r > 0 {
            let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
            let mut drawline = |sx: i32, ex: i32, y: i32| {
                for x in sx..=ex {
                    self.draw(x, y, pixel);
                }
            };

            while y0 >= x0 {
                drawline(x - y0, x + y0, y - x0);
                if x0 > 0 {
                    drawline(x - y0, x + y0, y + x0);
                }

                if d < 0 {
                    d += 4 * x0 + 6;
                    x0 += 1;
                } else {
                    if x0 != y0 {
                        drawline(x - x0, x + x0, y - y0);
                        drawline(x - x0, x + x0, y + y0);
                    }
                    d += 4 * (x0 - y0) + 10;
                    x0 += 1;
                    y0 -= 1;
                }
            }
        } else {
            self.draw(x, y, pixel);
        }
    }
    pub fn draw_rect(&mut self, pos: Vi2d, size: Vi2d, p: Pixel) {
        self.draw_rect_xy(pos.x, pos.y, size.x, size.y, p);
    }
    pub fn draw_rect_xy(&mut self, x: i32, y: i32, w: i32, h: i32, pixel: Pixel) {
        self.draw_line_xy(x, y, x + w, y, pixel);
        self.draw_line_xy(x + w, y, x + w, y + h, pixel);
        self.draw_line_xy(x + w, y + h, x, y + h, pixel);
        self.draw_line_xy(x, y + h, x, y, pixel);
    }

    pub fn draw_triangle(&mut self, pos1: Vf2d, pos2: Vf2d, pos3: Vf2d, p: Pixel) {
        self.draw_triangle_xy(
            pos1.x as i32,
            pos1.y as i32,
            pos2.x as i32,
            pos2.y as i32,
            pos3.x as i32,
            pos3.y as i32,
            p,
        );
    }

    pub fn draw_triangle_xy(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        p: Pixel,
    ) {
        self.draw_line_xy(x1, y1, x2, y2, Pixel::GREEN);
        self.draw_line_xy(x2, y2, x3, y3, Pixel::RED);
        self.draw_line_xy(x3, y3, x1, y1, Pixel::BLUE);
    }

    pub fn fill_triangle(&mut self, mut pos1: Vf2d, mut pos2: Vf2d, mut pos3: Vf2d, p: Pixel) {
        //Sort the points so that y1 <= y2 <= y3
        if pos2.y < pos1.y {
            std::mem::swap(&mut pos2, &mut pos1);
        }
        if pos3.y < pos1.y {
            std::mem::swap(&mut pos3, &mut pos1);
        }
        if pos3.y < pos2.y {
            std::mem::swap(&mut pos3, &mut pos2);
        }
        //This takes two vectors and a y position and returns the x coordinate
        let interpolate = |l: &Vf2d, r: &Vf2d, y: f32| -> f32 {
            let m = (r.x - l.x) / (r.y - l.y);
            (m * (y - l.y)) + l.x
        };

        for y in pos1.y.ceil() as i32..pos3.y.ceil() as i32 {
            let (mut start_x, mut end_x);
            //Fill the top half of the triangle
            if y < pos2.y.ceil() as i32 {
                start_x = interpolate(&pos1, &pos2, y as f32);
            } else {
                start_x = interpolate(&pos2, &pos3, y as f32);
            }
            end_x = interpolate(&pos1, &pos3, y as f32);
            if end_x - start_x < 0.0 {
                std::mem::swap(&mut start_x, &mut end_x);
            }
            for x in start_x as i32..end_x as i32 {
                self.draw(x, y, p);
            }
        }
    }

    pub fn texture_triangle(
        &mut self,
        mut pos1: Vf2d,
        mut pos2: Vf2d,
        mut pos3: Vf2d,
        mut uv1: UV,
        mut uv2: UV,
        mut uv3: UV,
        spr: Option<&Sprite>,
        tp: Pixel,
    ) {
        //Sort the points so that y1 <= y2 <= y3
        if pos2.y < pos1.y {
            std::mem::swap(&mut pos2, &mut pos1);
            std::mem::swap(&mut uv2, &mut uv1);
        }
        if pos3.y < pos1.y {
            std::mem::swap(&mut pos3, &mut pos1);
            std::mem::swap(&mut uv3, &mut uv1);
        }
        if pos3.y < pos2.y {
            std::mem::swap(&mut pos3, &mut pos2);
            std::mem::swap(&mut uv3, &mut uv2);
        }

        //This takes two vectors and a y position and returns the x coordinate
        let interpolate = |l: &Vf2d, r: &Vf2d, y: f32| -> f32 {
            let m = (r.x - l.x) / (r.y - l.y);
            (m * (y - l.y)) + l.x
        };

        //This takes two UVs and a y position and returns the x coordinate
        let interpolate_uv = |l: &UV, r: &UV, p: f32| -> UV {
            let d_u = r.u - l.u;
            let d_v = r.v - l.v;
            let d_w = r.w - l.w;
            ((d_u * p) + l.u, (d_v * p) + l.v, (d_w * p) + l.w).into()
        };
        pos1.x = pos1.x.min(self.pixels_w as f32);
        pos1.x = pos1.x.max(0.0);
        pos1.y = pos1.y.min(self.pixels_h as f32);
        pos1.y = pos1.y.max(0.0);

        pos2.x = pos2.x.min(self.pixels_w as f32);
        pos2.x = pos2.x.max(0.0);
        pos2.y = pos2.y.min(self.pixels_h as f32);
        pos2.y = pos2.y.max(0.0);

        pos3.x = pos3.x.min(self.pixels_w as f32);
        pos3.x = pos3.x.max(0.0);
        pos3.y = pos3.y.min(self.pixels_h as f32);
        pos3.y = pos3.y.max(0.0);

        for y in pos1.y.ceil() as i32..pos3.y.ceil() as i32 {
            let mut p = Pixel::BLANK;
            let dy_p12 = (y as f32 - pos1.y) / (pos2.y - pos1.y);
            let dy_p23 = (y as f32 - pos2.y) / (pos3.y - pos2.y);
            let dy_p13 = (y as f32 - pos1.y) / (pos3.y - pos1.y);
            let (mut start_x, mut end_x, mut start_uv, mut end_uv);
            //Fill the top half of the triangle
            if y < pos2.y.ceil() as i32 {
                start_x = interpolate(&pos1, &pos2, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);

                start_uv = interpolate_uv(&uv1, &uv2, dy_p12);
            } else {
                start_x = interpolate(&pos2, &pos3, y as f32);
                end_x = interpolate(&pos1, &pos3, y as f32);

                start_uv = interpolate_uv(&uv2, &uv3, dy_p23);
            }
            end_uv = interpolate_uv(&uv1, &uv3, dy_p13);

            if end_x - start_x < 0.0 {
                std::mem::swap(&mut start_x, &mut end_x);
                std::mem::swap(&mut start_uv, &mut end_uv);
            }
            for x in start_x as i32..end_x as i32 {
                let r = 1.0 - ((end_x as f64 - x as f64) / (end_x as f64 - start_x as f64));
                let d_uv: UV = (
                    end_uv.u - start_uv.u,
                    end_uv.v - start_uv.v,
                    end_uv.w - start_uv.w,
                )
                    .into();
                let dw = start_uv.w as f64 + (d_uv.w as f64 * r);
                if dw > self.depth_buffer[(y * self.pixels_w as i32 + x) as usize] {
                    let mut p = Pixel::default();
                    if let Some(sp) = spr {
                        p = sp.sample(
                            ((start_uv.u as f64 + (d_uv.u as f64 * r)) / dw) as f32,
                            ((start_uv.v as f64 + (d_uv.v as f64 * r)) / dw) as f32,
                        );
                    } else {
                        p = tp;
                    }
                    self.draw(x, y, p);
                    /*if wire && (x == start_x as i32 || x == (end_x as i32 - 1)
                        || y == pos1.y.ceil() as i32 || y == (pos3.y.ceil() as i32 - 1)) {
                        self.draw(x, y, Pixel::BLACK);
                    }*/
                    self.depth_buffer[(y * self.pixels_w as i32 + x) as usize] = dw;
                }
            }
        }
    }

    pub fn clear(&mut self, p: Pixel) {
        let (h, w) = (
            self.get_draw_target_height() as u32,
            self.get_draw_target_width() as u32,
        );
        if let LayerInfo::Image(image_data) = &mut self
            .get_draw_target()
            .expect("Can't get draw target")
            .layer_info
        {
            let pixels = h * w;
            image_data.sprite.clear(p);
        }
    }

    pub fn construct_font_sheet(&mut self) {
        let mut data: String = "".to_string();
        data += "?Q`0001oOch0o01o@F40o0<AGD4090LAGD<090@A7ch0?00O7Q`0600>00000000";
        data += "O000000nOT0063Qo4d8>?7a14Gno94AA4gno94AaOT0>o3`oO400o7QN00000400";
        data += "Of80001oOg<7O7moBGT7O7lABET024@aBEd714AiOdl717a_=TH013Q>00000000";
        data += "720D000V?V5oB3Q_HdUoE7a9@DdDE4A9@DmoE4A;Hg]oM4Aj8S4D84@`00000000";
        data += "OaPT1000Oa`^13P1@AI[?g`1@A=[OdAoHgljA4Ao?WlBA7l1710007l100000000";
        data += "ObM6000oOfMV?3QoBDD`O7a0BDDH@5A0BDD<@5A0BGeVO5ao@CQR?5Po00000000";
        data += "Oc``000?Ogij70PO2D]??0Ph2DUM@7i`2DTg@7lh2GUj?0TO0C1870T?00000000";
        data += "70<4001o?P<7?1QoHg43O;`h@GT0@:@LB@d0>:@hN@L0@?aoN@<0O7ao0000?000";
        data += "OcH0001SOglLA7mg24TnK7ln24US>0PL24U140PnOgl0>7QgOcH0K71S0000A000";
        data += "00H00000@Dm1S007@DUSg00?OdTnH7YhOfTL<7Yh@Cl0700?@Ah0300700000000";
        data += "<008001QL00ZA41a@6HnI<1i@FHLM81M@@0LG81?O`0nC?Y7?`0ZA7Y300080000";
        data += "O`082000Oh0827mo6>Hn?Wmo?6HnMb11MP08@C11H`08@FP0@@0004@000000000";
        data += "00P00001Oab00003OcKP0006@6=PMgl<@440MglH@000000`@000001P00000000";
        data += "Ob@8@@00Ob@8@Ga13R@8Mga172@8?PAo3R@827QoOb@820@0O`0007`0000007P0";
        data += "O`000P08Od400g`<3V=P0G`673IP0`@3>1`00P@6O`P00g`<O`000GP800000000";
        data += "?P9PL020O`<`N3R0@E4HC7b0@ET<ATB0@@l6C4B0O`H3N7b0?P01L3R000000020";

        let mut font_sprite = Sprite::new(128, 48);
        let mut py = 0;
        let mut px = 0;
        let mut data_chars: [u8; 1024] = [0; 1024];
        for (i, c) in data.chars().enumerate() {
            data_chars[i] = c as u8;
        }
        for b in (0..1024).step_by(4) {
            let sym1: u32 = (data_chars[b] as u32) - 48;
            let sym2: u32 = (data_chars[b + 1] as u32) - 48;
            let sym3: u32 = (data_chars[b + 2] as u32) - 48;
            let sym4: u32 = (data_chars[b + 3] as u32) - 48;
            let r: u32 = sym1 << 18 | sym2 << 12 | sym3 << 6 | sym4;

            for i in 0..24 {
                let k: u8 = if r & (1 << i) > 0 { 255 } else { 0 };
                font_sprite.set_pixel(px, py, Pixel::rgba(k, k, k, k));
                py += 1;
                if py == 48 {
                    px += 1;
                    py = 0;
                }
            }
        }
        //self.font_decal = Decal::create(Some(font_sprite), &mut self.renderer);
    }

    pub fn draw_decal(&mut self, pos: Vf2d, decal: Arc<SmallD>) {
        self.draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0, 1.0), Pixel::WHITE);
    }

    pub fn draw_decal_with_scale(&mut self, pos: Vf2d, decal: Arc<SmallD>, scale: Vf2d) {
        self.draw_decal_with_scale_and_tint(pos, decal, scale, Pixel::WHITE);
    }
    pub fn draw_decal_with_tint(&mut self, pos: Vf2d, decal: Arc<SmallD>, tint: Pixel) {
        self.draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0, 1.0), tint);
    }

    pub fn draw_decal_with_scale_and_tint(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let screen_space_pos = Vf2d::from((
            (pos.x * self.inv_screen_size.x) * 2.0 - 1.0,
            ((pos.y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
        ));
        let screen_space_dim = Vf2d::from((
            screen_space_pos.x + (2.0 * (decal.sprite.width as f32) * self.inv_screen_size.x),
            screen_space_pos.y - (2.0 * (decal.sprite.height as f32) * self.inv_screen_size.y),
        ));
        let mut di = DecalInstance {
            decal: Some(decal),
            ..Default::default()
        };
        di.tint[0] = tint;
        di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
        di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
        di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
        di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));
        self.push_decal_instance(di);
    }

    pub fn draw_partial_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        source_pos: Vf2d,
        source_size: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let screen_space_pos = Vf2d::from((
            (pos.x * self.inv_screen_size.x) * 2.0 - 1.0,
            ((pos.y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
        ));
        let screen_space_dim = Vf2d::from((
            screen_space_pos.x + (2.0 * (source_size.x as f32) * self.inv_screen_size.x) * scale.x,
            screen_space_pos.y - (2.0 * (source_size.y as f32) * self.inv_screen_size.y) * scale.y,
        ));
        let mut di = DecalInstance::default();
        di.tint[0] = tint;

        di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
        di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
        di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
        di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));

        let uvtl = Vf2d::from((
            source_pos.x * decal.uv_scale.x,
            source_pos.y * decal.uv_scale.y,
        ));

        let uvbr = Vf2d::from((
            uvtl.x + (source_size.x * decal.uv_scale.x),
            uvtl.y + (source_size.y * decal.uv_scale.y),
        ));

        di.uv[0] = Vf2d::from((uvtl.x, uvtl.y));
        di.uv[1] = Vf2d::from((uvtl.x, uvbr.y));
        di.uv[2] = Vf2d::from((uvbr.x, uvbr.y));
        di.uv[3] = Vf2d::from((uvbr.x, uvtl.y));
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }

    pub fn draw_rotated_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        angle: f32,
        center: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::default();
        di.tint[0] = tint;
        di.pos[0] = Vf2d::new(0.0 - center.x * scale.x, 0.0 - center.y * scale.y);
        di.pos[1] = Vf2d::new(
            0.0 - center.x * scale.x,
            decal.sprite.height as f32 - center.y * scale.y,
        );
        di.pos[2] = Vf2d::new(
            decal.sprite.width as f32 - center.x * scale.x,
            decal.sprite.height as f32 - center.y * scale.y,
        );
        di.pos[3] = Vf2d::new(
            decal.sprite.width as f32 - center.x * scale.x,
            0.0 - center.y * scale.y,
        );
        let (c, s) = (angle.cos(), angle.sin());
        for i in 0..4 {
            di.pos[i] = Vf2d::new(
                di.pos[1].x * c - di.pos[i].y * s,
                di.pos[i].x * s + di.pos[i].y * c,
            );
            di.pos[i] = Vf2d::new(
                di.pos[i].x * self.inv_screen_size.x * 2.0 - 1.0,
                di.pos[i].y * self.inv_screen_size.y * 2.0 - 1.0,
            );
            di.pos[i].y += -1.0;
        }
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }

    pub fn draw_warped_decal(&mut self, decal: Arc<SmallD>, pos: &[Vf2d]) {
        self.draw_warped_decal_with_tint(decal, pos, Pixel::WHITE);
    }

    pub fn draw_warped_decal_with_tint(&mut self, decal: Arc<SmallD>, pos: &[Vf2d], tint: Pixel) {
        let mut di = DecalInstance {
            decal: Some(decal),
            ..Default::default()
        };
        di.tint[0] = tint;
        let mut center = Vf2d::new(0.0, 0.0);
        let mut rd: f32 = (pos[2].x - pos[0].x) * (pos[3].y - pos[1].y)
            - (pos[3].x - pos[1].x) * (pos[2].y - pos[0].y);
        if rd != 0.0 {
            rd = 1.0 / rd;
            let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y)
                - (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x))
                * rd;
            let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y)
                - (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x))
                * rd;
            if !((0.0..=1.0).contains(&rn) || (0.0..=1.0).contains(&sn)) {
                let i = pos[2] - pos[0];
                center = pos[0] + Vf2d::new(rn * i.x, rn * i.y);
            }
            let mut d: [f32; 4] = [0.0; 4];
            for i in 0..4 {
                d[i] = (pos[i] - center).mag();
            }
            for i in 0..4 {
                let q = if d[i] == 0.0 {
                    1.0
                } else {
                    (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3]
                };
                di.uv[i].x *= q;
                di.uv[i].y *= q;
                di.w[i] *= q;
                di.pos[i] = Vf2d::new(
                    (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                    ((pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
                );
            }
            self.push_decal_instance(di);
        }
    }

    pub fn draw_partial_warped_decal(
        &mut self,
        decal: Arc<SmallD>,
        pos: Vec<Vf2d>,
        source_pos: Vf2d,
        source_size: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::default();
        di.tint[0] = tint;
        let mut center = Vf2d::new(0.0, 0.0);
        let mut rd: f32 = (pos[2].x - pos[0].x) * (pos[3].y - pos[1].y)
            - (pos[3].x - pos[1].x) * (pos[2].y - pos[0].y);
        if rd != 0.0 {
            let uvtl = Vf2d::new(
                source_pos.x * decal.uv_scale.x,
                source_pos.y * decal.uv_scale.y,
            );

            let uvbr = Vf2d::new(
                uvtl.x + (source_size.x * decal.uv_scale.x),
                uvtl.y + (source_size.y * decal.uv_scale.y),
            );

            di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
            di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
            di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
            di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
            rd = 1.0 / rd;
            let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y)
                - (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x))
                * rd;
            let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y)
                - (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x))
                * rd;
            if !((0.0..=1.0).contains(&rn) || (0.0..=1.0).contains(&sn)) {
                let i = pos[2] - pos[0];
                center = Vf2d::new(pos[0].x + rn, pos[0].y + rn) * i;
            }
            let mut d: [f32; 4] = [0.0; 4];
            for i in 0..4 {
                d[i] = (pos[i] - center).mag();
            }
            for i in 0..4 {
                let q = if d[i] == 0.0 {
                    1.0
                } else {
                    (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3]
                };
                di.uv[i].x *= q;
                di.uv[i].y *= q;
                di.w[i] *= q;
                di.pos[i] = Vf2d::new(
                    (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                    ((pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0) * -1.0,
                );
            }
            di.decal = Some(decal);
            self.push_decal_instance(di);
        }
    }

    pub fn draw_explicit_decal(
        &mut self,
        decal: Arc<SmallD>,
        pos: Vec<Vf2d>,
        uv: Vec<Vf2d>,
        col: Vec<Pixel>,
    ) {
        let mut di = DecalInstance::default();
        if decal.id > 0 {
            di.decal = Some(decal);
        } else {
            di.decal = None;
        }

        for i in 0..4 {
            di.pos[i] = Vf2d::from((
                (pos[i].x * self.inv_screen_size.x) * 2.0 - 1.0,
                (pos[i].y * self.inv_screen_size.y) * 2.0 - 1.0,
            ));
            di.uv[i] = uv[i];
            di.tint[i] = col[i];
        }
        self.push_decal_instance(di);
    }

    pub fn fill_rect_decal(&mut self, pos: Vf2d, size: Vf2d, col: Pixel) {
        let points = vec![
            pos,
            Vf2d::new(pos.x, pos.y + size.y),
            pos + size,
            Vf2d::new(pos.x + size.x, pos.y),
        ];
        let uvs = vec![(1.0, 1.0).into(); 4];
        let cols = vec![col, col, col, col];
        self.draw_explicit_decal(Decal::empty().get(), points, uvs, cols);
    }

    pub fn gradient_fill_rect_decal(
        &mut self,
        pos: Vf2d,
        size: Vf2d,
        colTL: Pixel,
        colBL: Pixel,
        colTR: Pixel,
        colBR: Pixel,
    ) {
        let points = vec![
            pos,
            Vf2d::new(pos.x, pos.y + size.y),
            pos + size,
            Vf2d::new(pos.x + size.x, pos.y),
        ];
        let uvs = vec![(1.0, 1.0).into(); 4];
        let cols = vec![colTL, colBL, colBR, colTR];
        self.draw_explicit_decal(Decal::empty().get(), points, uvs, cols);
    }

    pub fn draw_partial_rotated_decal(
        &mut self,
        pos: Vf2d,
        decal: Arc<SmallD>,
        angle: f32,
        center: Vf2d,
        source_pos: Vf2d,
        source_size: Vf2d,
        scale: Vf2d,
        tint: Pixel,
    ) {
        let mut di = DecalInstance::default();
        di.tint[0] = tint;
        di.pos[0] = (Vf2d::new(0.0, 0.0) - center) * scale;
        di.pos[1] = (Vf2d::new(0.0, source_size.y) - center) * scale;
        di.pos[2] = (Vf2d::new(source_size.x, source_size.y) - center) * scale;
        di.pos[3] = (Vf2d::new(source_size.x, 0.0) - center) * scale;
        let (c, s) = (angle.cos(), angle.sin());
        for i in 0..4 {
            di.pos[i] = Vf2d::new(
                di.pos[1].x * c - di.pos[i].y * s,
                di.pos[i].x * s + di.pos[i].y * c,
            );
            di.pos[i] = Vf2d::new(
                di.pos[i].x * self.inv_screen_size.x * 2.0 - 1.0,
                di.pos[i].y * self.inv_screen_size.y * 2.0 - 1.0,
            );
            di.pos[i].y += -1.0;
        }
        let uvtl = Vf2d::new(
            source_pos.x * decal.uv_scale.x,
            source_pos.y * decal.uv_scale.y,
        );

        let uvbr = Vf2d::new(
            uvtl.x + (source_size.x * decal.uv_scale.x),
            uvtl.y + (source_size.y * decal.uv_scale.y),
        );

        di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
        di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
        di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
        di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
        di.decal = Some(decal);
        self.push_decal_instance(di);
    }
    pub fn draw_string_decal(&mut self, pos: Vf2d, text: &str) {
        self.draw_string_decal_with_color_and_scale(pos, text, Pixel::WHITE, Vf2d::new(1.0, 1.0));
    }

    pub fn draw_string_decal_with_color(&mut self, pos: Vf2d, text: &str, col: Pixel) {
        self.draw_string_decal_with_color_and_scale(pos, text, col, Vf2d::new(1.0, 1.0));
    }

    pub fn draw_string_decal_with_scale(&mut self, pos: Vf2d, text: &str, scale: Vf2d) {
        self.draw_string_decal_with_color_and_scale(pos, text, Pixel::WHITE, scale);
    }

    pub fn draw_string_decal_with_color_and_scale(
        &mut self,
        pos: Vf2d,
        text: &str,
        col: Pixel,
        scale: Vf2d,
    ) {
        //self.draw_decal(pos, &FONT_DECAL, scale, Pixel::WHITE);
        let mut spos = Vf2d::new(0.0, 0.0);
        for c in text.chars() {
            if c == '\n' {
                spos.x = 0.0;
                spos.y += 8.0 * scale.y;
            } else {
                let ox = (c as u8 - 32) % 16;
                let oy = (c as u8 - 32) / 16;
                self.draw_partial_decal(
                    pos + spos,
                    self.font_decal.get(),
                    Vf2d::new(ox as f32 * 8.0, oy as f32 * 8.0),
                    Vf2d::new(8.0, 8.0),
                    scale,
                    col,
                );
                spos.x += 8.0 * scale.x;
            }
        }
    }

    pub fn get_text_size(&self, s: String) -> Vi2d {
        let (mut size, mut pos) = (Vi2d::new(0, 1), Vi2d::new(0, 1));
        for c in s.chars() {
            if c == '\n' {
                pos.y += 1;
                pos.x = 0;
            } else {
                pos.x += 1
            }
            size.x = std::cmp::max(size.x, pos.x);
            size.y = std::cmp::max(size.y, pos.y);
        }
        Vi2d::new(size.x * 8, size.y * 8)
    }
}

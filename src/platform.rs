use super::{
    olc::Rcode,
    util::{Vi2d, Vf2d, HWButton},
};
use std::collections::hash_map::HashMap;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use winit::{
    event::{Event, WindowEvent, ElementState, MouseScrollDelta},
    event_loop::EventLoop,
    window::{Window,WindowBuilder},
};

pub const MOUSE_BUTTONS: u8 = 5;

pub trait Platform {
    fn create_window_pane(
        window_pos: Vi2d,
        window_size: Vi2d,
        full_screen: bool,
    ) -> (Window, EventLoop<()>);
    fn application_startup(&self) -> Rcode {
        Rcode::Ok
    }
    fn application_cleanup(&self) -> Rcode {
        Rcode::Ok
    }
    fn thread_startup(&self) -> Rcode {
        Rcode::Ok
    }
    fn thread_cleanup(&self) -> Rcode {
        Rcode::Ok
    }
    fn create_graphics(
        &mut self,
        full_screen: bool,
        enable_vsync: bool,
        view_pos: Vi2d,
        view_size: Vi2d,
    ) -> Rcode {
        Rcode::Ok
    }
    fn set_window_title(window: &Window, title: String) -> Rcode {
        Rcode::Ok
    }
    fn handle_window_event(window: &Window, event: &Event<()>);
    fn handle_system_event_loop(&self) -> Rcode {
        Rcode::Ok
    }
    fn handle_system_event(&self) -> Rcode {
        Rcode::Ok
    }
}

//#[cfg(not(target_arch = "wasm32"))]
pub struct PlatformWindows {
    pub window: Window,
    pub event_loop: EventLoop<()>,
}
/*
#[cfg(target_arch = "wasm32")]
pub struct PlatformWeb {
    window: Window,
    event_loop: EventLoop<()>,
}
*/

pub static mut PLATFORM_DATA: PlatformData = PlatformData::create();

//this is only ever updated from the Platform thread,
// so immutable references to it are thread safe
pub type Key = winit::event::VirtualKeyCode;
pub struct PlatformData {
    pub mouse_focus: bool,
    pub key_focus: bool,
    pub new_key_state_map: Option<HashMap<Key, bool>>,
    pub old_key_state_map: Option<HashMap<Key, bool>>,
    pub new_mouse_state_map: Option<Vec<bool>>,
    pub old_mouse_state_map: Option<Vec<bool>>,
    pub key_map: Option<HashMap<Key, HWButton>>,
    pub mouse_map: Option<Vec<HWButton>>,
    pub mouse_wheel_delta: i32,
    pub mouse_wheel_delta_cache: i32,
    pub mouse_position: Option<Vf2d>,
    pub raw_mouse_position: Option<Vf2d>,
    pub view_position: Option<Vi2d>,
    pub window_size: Option<Vi2d>,
    pub resolution: Option<Vi2d>,
    pub screen_size: Option<Vi2d>,
    pub pixel_size: Option<Vi2d>,
    pub mouse_position_cache: Option<Vf2d>,
    pub window_alive: bool,
    pub full_screen: bool,
    pub vsync: bool,
    pub title: String,
    pub running: bool,
    pub y_up_direction: f32,
}

impl PlatformData {
    pub const fn create() -> Self {
        #[cfg(target_arch="wasm32")]
        let y_up_direction = -1.0;
        #[cfg(not(target_arch="wasm32"))]
        let y_up_direction = 1.0;
        Self {
            mouse_focus: false,
            key_focus: false,
            new_key_state_map: None,
            old_key_state_map: None,
            new_mouse_state_map: None,
            old_mouse_state_map: None,
            key_map: None,
            mouse_map: None,
            mouse_wheel_delta: 0,
            mouse_wheel_delta_cache: 0,
            mouse_position: None,
            raw_mouse_position: None,
            view_position: None,
            window_size: None,
            pixel_size: None,
            resolution: None,
            screen_size: None,
            mouse_position_cache: None,
            window_alive: true,
            full_screen: false,
            vsync: false,
            title: String::new(),
            running: true,
            y_up_direction,
        }
    }
    pub fn init(&mut self) {
        self.new_key_state_map = Some(HashMap::default());
        self.old_key_state_map = Some(HashMap::default());
        self.new_mouse_state_map = Some(vec![false; 3]);
        self.old_mouse_state_map = Some(vec![false; 3]);
        self.key_map = Some(HashMap::default());
        self.mouse_map = Some(vec![HWButton::new(); 3]);
        self.mouse_position = Some((0.0, 0.0).into());
        self.view_position = Some((0, 0).into());
        self.window_size = Some((0, 0).into());
        self.resolution = Some((0, 0).into());
        self.screen_size = Some((0, 0).into());
        self.pixel_size = Some((0, 0).into());
        self.mouse_position_cache = Some((0.0, 0.0).into());
    }

    pub fn update_mouse(&mut self, mut x: i32, mut y: i32) {
        self.raw_mouse_position = Some(
            (
                x as f32 + self.view_position.unwrap_or_default().x as f32,
                y as f32 + self.view_position.unwrap_or_default().y as f32,
            )
                .into(),
        );
        self.mouse_focus = true;
        let px_i = self.pixel_size.unwrap_or_default();
        let px: Vf2d = (px_i.x as f32, px_i.y as f32).into();
        let mut temp_mouse: Vi2d = ((x as f32 / px.x) as i32, (y as f32 / px.y) as i32).into();
        if temp_mouse.x >= (self.window_size.unwrap_or_default().x as f32 / px.x).floor() as i32 {
            temp_mouse.x = (self.window_size.unwrap_or_default().x as f32 / px.x) as i32 - 1
        }
        if temp_mouse.y >= (self.window_size.unwrap_or_default().y as f32 / px.y).floor() as i32 {
            temp_mouse.y = (self.window_size.unwrap_or_default().y as f32 / px.y) as i32 - 1
        }
        //log::trace!("{:x?}", temp_mouse);
        if temp_mouse.x < 0 {
            temp_mouse.x = 0
        }
        if temp_mouse.y < 0 {
            temp_mouse.y = 0
        }
        self.mouse_position_cache = Some((temp_mouse.x as f32, temp_mouse.y as f32).into());
    }
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = Some(Vi2d::from(((width as i32), (height as i32))));
    }
    pub fn update_window_position(&mut self, x: i32, y: i32) {
        self.view_position = Some(Vi2d::from(((x as i32), (y as i32))));
    }
    pub fn update_mouse_wheel(&mut self, delta: i32) {
        self.mouse_wheel_delta_cache += delta;
    }
    pub fn update_mouse_focus(&mut self, b: bool) {
        self.mouse_focus = b
    }
    pub fn update_key_focus(&mut self, b: bool) {
        self.key_focus = b
    }
    pub fn update_key_state(&mut self, k: Key, b: bool) {
        *self
            .new_key_state_map
            .as_mut()
            .unwrap()
            .entry(k)
            .or_insert(b) = b;
    }
    pub fn update_mouse_state(&mut self, i: i32, b: bool) {
        self.new_mouse_state_map.as_mut().unwrap()[i as usize] = b;
    }
}

//#[cfg(not(target_arch = "wasm32"))]
impl PlatformWindows {
    pub fn new() -> PlatformWindows {
        let event_loop = EventLoop::new();
        PlatformWindows {
            window: WindowBuilder::new().build(&event_loop).unwrap(),
            event_loop,
        }
    }
}

//#[cfg(not(target_arch = "wasm32"))]
impl Platform for PlatformWindows {
    fn create_window_pane(
        window_pos: Vi2d,
        window_size: Vi2d,
        full_screen: bool,
    ) -> (Window, EventLoop<()>) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::Size::Logical(winit::dpi::LogicalSize {
                width: window_size.x as f64,
                height: window_size.y as f64,
            }))
            /*.with_fullscreen(
            Some(winit::window::Fullscreen::Borderless(
            event_loop.available_monitors().next().expect("Wrong monitor"))))*/
            .build(&event_loop)
            .expect("Failed to build Window");

        (window, event_loop)
    }

    fn set_window_title(window: &Window, title: String) -> Rcode {
        window.set_title(&title);
        unsafe { PLATFORM_DATA.title = title }
        Rcode::Ok
    }

    fn handle_window_event(window: &Window, event: &Event<()>) {
        unsafe {
            if let Event::WindowEvent {
                window_id: _,
                ref event,
            } = event
            {
                match event {
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                        modifiers: _,
                    } => {
                        PLATFORM_DATA.update_mouse(position.x as i32, position.y as i32);
                    }
                    WindowEvent::Resized(size) => {
                        PLATFORM_DATA.update_window_size(size.width, size.height);
                    }
                    WindowEvent::Moved(position) => {
                        PLATFORM_DATA.update_window_position(position.x, position.y);
                    }
                    WindowEvent::MouseWheel {
                        device_id: _,
                        delta: MouseScrollDelta::LineDelta(h, v),
                        phase,
                        modifiers: _,
                    } => {
                        PLATFORM_DATA.update_mouse_wheel(*v as i32);
                    }
                    WindowEvent::CursorLeft { device_id: _ } => {
                        PLATFORM_DATA.update_mouse_focus(false);
                    }
                    WindowEvent::Focused(focus) => {
                        PLATFORM_DATA.update_key_focus(*focus);
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic,
                    } => {
                        if let Some(key) = input.virtual_keycode {
                            PLATFORM_DATA
                                .update_key_state(key, input.state == ElementState::Pressed);
                        }
                    }
                    WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                        modifiers: _,
                    } => match button {
                        winit::event::MouseButton::Left => {
                            PLATFORM_DATA.update_mouse_state(0, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Right => {
                            PLATFORM_DATA.update_mouse_state(1, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Middle => {
                            PLATFORM_DATA.update_mouse_state(2, state == &ElementState::Pressed)
                        }
                        winit::event::MouseButton::Other(b) => PLATFORM_DATA
                            .update_mouse_state(*b as i32, state == &ElementState::Pressed),
                    },
                    _ => {}
                }
            }
        }
    }

    fn handle_system_event(&self) -> Rcode {
        Rcode::Fail
    }
}

#![feature(c_variadic)]
#![allow(non_snake_case)]
#![feature(once_cell)]
//#![feature(nll)]

#[cfg(windows)]
extern crate winapi;

#[cfg(windows)]
use self::winapi::um::winuser::*;
#[cfg(windows)]
use self::winapi::shared::windef::HWND;


use core::{ops, fmt};
use super::gl_const::*;
use std::fmt::Error;
use std::rc::Rc;
use self::winapi::shared::minwindef::{HINSTANCE__, WPARAM, LPARAM, LRESULT, DWORD, HINSTANCE, HMODULE};
use self::winapi::um::libloaderapi::{GetModuleHandleA, GetModuleHandleW, GetProcAddress, LoadLibraryA, LoadLibraryExW};
use self::winapi::shared::ntdef::{LPCSTR, LPCWSTR, NULL};
use self::winapi::shared::windef::{HBRUSH__, RECT, HMENU__, HMONITOR__, HWND__, HMONITOR, HMENU, HICON, HBRUSH, POINT, HDC, HGLRC, HGLRC__, HDC__};
use self::winapi::ctypes::{c_void, c_char};
use std::ffi::{CString, CStr};
use widestring::WideCString;
use std::mem::{size_of, MaybeUninit};
use std::ptr::null;
use self::winapi::um::wingdi::{PIXELFORMATDESCRIPTOR, PFD_DRAW_TO_WINDOW,
                               PFD_DOUBLEBUFFER, PFD_SUPPORT_OPENGL,
                               PFD_TYPE_RGBA, PFD_MAIN_PLANE, ChoosePixelFormat,
                               SetPixelFormat, wglCreateContext, wglMakeCurrent,
                               wglGetProcAddress, wglDeleteContext, SwapBuffers};
use self::winapi::um::dwmapi::DwmFlush;
use std::ops::Try;
use std::{thread, mem};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::cell::RefCell;
use std::borrow::Borrow;
use std::time::{Duration, UNIX_EPOCH, SystemTime};
use std::thread::sleep;
use std::sync::mpsc::channel;
use pretty_hex::*;
use rand::Rng;
use rand::prelude::ThreadRng;
use lazy_static::*;
use std::collections::HashMap;
use ops::{DerefMut, Deref};
use std::lazy::SyncLazy;
use std::lazy::OnceCell;

const MOUSE_BUTTONS: u8 = 5;
const DEFAULT_ALPHA: u8 = 0xFF;
const DEFAULT_PIXEL: u32 = 0xFF << 24;


struct PGEBox<T>(T);
impl<T> PGEBox<T>{
    fn new(x: T) -> Self{
        PGEBox(x)
    }
}

impl<T> Deref for PGEBox<T>{
    type Target = T;
    fn deref(&self) -> &T{
        &self.0
    }
}
impl<T> DerefMut for PGEBox<T>{
    fn deref_mut(&mut self) -> &mut T{
        &mut self.0
    }
}

lazy_static! {
    static ref GL: GLLoader = GLLoader::construct();
    static ref FONT_DECAL: Decal = construct_font_sheet();
}
static mut PGE: OnceCell<OLCEngine> = OnceCell::new();


pub enum Rcode {
    Fail,
    Ok,
    NoFile,
}

impl Try for Rcode {
    type Ok = (Rcode);
    type Error = (Rcode);

    fn into_result(self) -> Result<Rcode, Rcode> {
        match self {
            Rcode::Ok => Ok(Rcode::Ok),
            Rcode::Fail => Err(Rcode::Fail),
            Rcode::NoFile => Err(Rcode::NoFile),
        }
    }

    fn from_error(v: Rcode) -> Self {
        v
    }

    fn from_ok(v: Rcode) -> Self {
        v
    }
}

pub struct Renderer {
    gl_device_context: HDC,
    gl_render_context: HGLRC,
    // gl_functions:
    vsync: bool,
}

impl Renderer {
    fn new() -> Self {
        Renderer {
            gl_device_context: NULL as HDC,
            gl_render_context: NULL as HGLRC,
            vsync: false,
        }
    }
    pub fn create_graphics(
        &mut self,
        full_screen: bool,
        enable_vsync: bool,
        view_pos: Vi2d,
        view_size: Vi2d,
    ) -> Rcode {
        unsafe {
            self.create_device(PLATFORM_INSTANCE, full_screen, enable_vsync)?;
            Renderer::update_viewport(view_pos, view_size)?;
        }
        (GL.glEnable)(GL_DEBUG_OUTPUT);
        (GL.glDebugMessageCallback)(gl_message_callback, 0);

        Rcode::Ok
    }
    pub fn create_device(&mut self, hwnd: HWND, full_screen: bool, vsync: bool) -> Rcode {
        unsafe {
            self.gl_device_context = GetDC(hwnd);
        }
        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 0,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 0,
            cStencilBits: 0,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };
        unsafe {
            let mut pf = ChoosePixelFormat(self.gl_device_context, &pfd);
            if pf == 0 { return Rcode::Fail; }
            SetPixelFormat(self.gl_device_context, pf, &pfd);
            self.gl_render_context = wglCreateContext(self.gl_device_context);
            wglMakeCurrent(self.gl_device_context, self.gl_render_context);
            if !vsync {
                (GL.wglSwapIntervalEXT)(0);
            }
            self.vsync = vsync;
        }
        (GL.glEnable)(GL_TEXTURE_2D);
        (GL.glHint)(GL_PERSPECTIVE_CORRECTION_HINT, GL_NICEST);
        Rcode::Ok
    }

    pub fn destroy_device(&mut self) -> Rcode {
        unsafe {
            wglDeleteContext(self.gl_render_context);
        }
        Rcode::Ok
    }

    pub fn update_viewport(position: Vi2d, size: Vi2d) -> Rcode {
        unsafe {
            PLATFORM_DATA.view_position = Some(position);
            PLATFORM_DATA.window_size = Some(size);
            PLATFORM_DATA.screen_size = Some(size);
        }
        (GL.glViewport)(position.x as u32, position.y as u32,
                        size.x as u32, size.y as u32);
        Rcode::Ok
    }
    pub fn display_frame(&self) -> Rcode {
        unsafe {
            SwapBuffers(self.gl_device_context);
            if self.vsync { DwmFlush(); }
        }
        Rcode::Ok
    }

    pub fn clear_buffer(p: Pixel, depth: bool) {
        unsafe {
            (GL.glClearColor)(p.rgba.0 as f32 / 255.0, p.rgba.1 as f32 / 255.0,
                              p.rgba.2 as f32 / 255.0, p.rgba.3 as f32 / 255.0, );
        }
        (GL.glClear)(GL_COLOR_BUFFER_BIT);
        if depth { (GL.glClear)(GL_DEPTH_BUFFER_BIT) }
    }

    pub fn prepare_drawing() {
        (GL.glEnable)(GL_BLEND);
        (GL.glBlendFunc)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    }

    pub fn create_texture(width: u32, height: u32) -> u32 {
        let mut id: u32 = 0;
        (GL.glGenTextures)(1, &mut id);
        (GL.glBindTexture)(GL_TEXTURE_2D, id);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAX_LEVEL, 0);
        (GL.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_BASE_LEVEL, 0);
        (GL.glTexEnvi)(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
        id
    }

    pub fn delete_texture(id: &mut u32) -> &mut u32 {
        (GL.glDeleteTextures)(1, id);
        id
    }

    pub fn apply_texture(id: u32) {
        (GL.glBindTexture)(GL_TEXTURE_2D, id);
    }

    pub fn update_texture(_id: u32, spr: &Sprite) {
        let mut sprite_pointer = spr.col_data.as_ptr() as *const usize;
        (GL.glTexImage2D)(GL_TEXTURE_2D, 0, GL_RGBA8, spr.width, spr.height, 0, GL_BGRA_EXT,
                          GL_UNSIGNED_BYTE, sprite_pointer);
    }
    pub fn draw_layer_quad(offset: Vf2d, scale: Vf2d, tint: Pixel) {
        //The () functions are because accessing the union is unsafe, and I
        // don't like leaving unsafe{} all over the place.
        (GL.glBegin)(GL_QUADS);
        (GL.glColor4ub)(tint.r(), tint.g(), tint.b(), tint.a());

        (GL.glTexCoord2f)(0.0 * scale.x + offset.x, 1.0 * scale.y + offset.y);
        //glColor4ub(255,0,255,255);
        (GL.glVertex2f)(-1.0, -1.0);

        (GL.glTexCoord2f)(0.0 * scale.x + offset.x, 0.0 * scale.y + offset.y);
        //glColor4ub(0,0,255,255);
        (GL.glVertex2f)(-1.0, 1.0);

        (GL.glTexCoord2f)(1.0 * scale.x + offset.x, 0.0 * scale.y + offset.y);
        //glColor4ub(0,255,255,255);
        (GL.glVertex2f)(1.0, 1.0);

        (GL.glTexCoord2f)(1.0 * scale.x + offset.x, 1.0 * scale.y + offset.y);
        //glColor4ub(0,255,0,255);
        (GL.glVertex2f)(1.0, -1.0);
        (GL.glEnd)();
    }

    pub fn draw_decal_quad(decal: &mut DecalInstance) {
        //I'm wrapping this whole thing in unsafe because
        // it accesses Union values
        unsafe {
            if let None = decal.decal {
                (GL.glBindTexture)(GL_TEXTURE_2D, 0);

                (GL.glBegin)(GL_QUADS);
                (GL.glColor4ub)(decal.tint[0].rgba.0, decal.tint[0].rgba.1, decal.tint[0].rgba.2, decal.tint[0].rgba.3);
                (GL.glTexCoord4f)(decal.uv[0].x, decal.uv[0].y, 0.0, decal.w[0]);
                (GL.glVertex2f)(decal.pos[0].x, decal.pos[0].y);
                (GL.glColor4ub)(decal.tint[1].rgba.0, decal.tint[1].rgba.1,
                                decal.tint[1].rgba.2, decal.tint[1].rgba.3);
                (GL.glTexCoord4f)(decal.uv[1].x, decal.uv[1].y, 0.0, decal.w[1]);
                (GL.glVertex2f)(decal.pos[1].x, decal.pos[1].y);
                (GL.glColor4ub)(decal.tint[2].rgba.0, decal.tint[2].rgba.1,
                                decal.tint[2].rgba.2, decal.tint[2].rgba.3);
                (GL.glTexCoord4f)(decal.uv[2].x, decal.uv[2].y, 0.0, decal.w[2]);
                (GL.glVertex2f)(decal.pos[2].x, decal.pos[2].y);
                (GL.glColor4ub)(decal.tint[3].rgba.0, decal.tint[3].rgba.1,
                                decal.tint[3].rgba.2, decal.tint[3].rgba.3);
                (GL.glTexCoord4f)(decal.uv[3].x, decal.uv[3].y, 0.0, decal.w[3]);
                (GL.glVertex2f)(decal.pos[3].x, decal.pos[3].y);
                (GL.glEnd)();
            } else {
                let decal_id = decal.get().id;
                (GL.glBindTexture)(GL_TEXTURE_2D, decal_id as u32);
                (GL.glBegin)(GL_QUADS);
                (GL.glColor4ub)(decal.tint[0].rgba.0, decal.tint[0].rgba.1,
                                decal.tint[0].rgba.2, decal.tint[0].rgba.3);
                (GL.glTexCoord4f)(decal.uv[0].x, decal.uv[0].y, 0.0, decal.w[0]);
                (GL.glVertex2f)(decal.pos[0].x, decal.pos[0].y);
                (GL.glTexCoord4f)(decal.uv[1].x, decal.uv[1].y, 0.0, decal.w[1]);
                (GL.glVertex2f)(decal.pos[1].x, decal.pos[1].y);
                (GL.glTexCoord4f)(decal.uv[2].x, decal.uv[2].y, 0.0, decal.w[2]);
                (GL.glVertex2f)(decal.pos[2].x, decal.pos[2].y);
                (GL.glTexCoord4f)(decal.uv[3].x, decal.uv[3].y, 0.0, decal.w[3]);
                (GL.glVertex2f)(decal.pos[3].x, decal.pos[3].y);
                (GL.glEnd)();
            }
        }
    }
}

pub trait Platform {
    fn application_startup(&self) -> Rcode { Rcode::Ok }
    fn application_cleanup(&self) -> Rcode { Rcode::Ok }
    fn thread_startup(&self) -> Rcode { Rcode::Ok }
    fn thread_cleanup(&self) -> Rcode { Rcode::Ok }
    fn create_graphics(&mut self, full_screen: bool, enable_vsync: bool,
                       view_pos: Vi2d, view_size: Vi2d) -> Rcode { Rcode::Ok }
    fn create_window_pane(&mut self, window_pos: Vi2d,
                          mut window_size: Vi2d,
                          full_screen: bool) -> Rcode { Rcode::Ok }
    fn set_window_title(&self, title: String) -> Rcode { Rcode::Ok }
    fn start_system_event_loop(&self) -> Rcode { Rcode::Ok }
    fn handle_system_event_loop(&self) -> Rcode { Rcode::Ok }
    fn handle_system_event(&self) -> Rcode { Rcode::Ok }
}

#[cfg(windows)]
pub type WindowEvent = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM)
                                                 -> LRESULT;

#[cfg(windows)]
pub struct PlatformWindows {
    hwnd: HWND,
    running: bool,
}

static mut PLATFORM_DATA: PlatformData = PlatformData::create();

//this is only ever updated from the Platform thread,
// so immutable references to it are thread safe
struct PlatformData {
    mouse_focus: bool,
    key_focus: bool,
    new_key_state_map: Option<Vec<bool>>,
    old_key_state_map: Option<Vec<bool>>,
    new_mouse_state_map: Option<Vec<bool>>,
    old_mouse_state_map: Option<Vec<bool>>,
    key_map: Option<Vec<HWButton>>,
    mouse_map: Option<Vec<HWButton>>,
    mouse_wheel_delta: i32,
    mouse_wheel_delta_cache: i32,
    mouse_position: Option<Vf2d>,
    view_position: Option<Vi2d>,
    window_size: Option<Vi2d>,
    screen_size: Option<Vi2d>,
    pixel_size: Option<Vi2d>,
    mouse_position_cache: Option<Vf2d>,
    window_alive: bool,
    full_screen: bool,
    vsync: bool,
    title: &'static str,
    running: bool,
}

impl PlatformData {
    const fn create() -> Self {
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
            view_position: None,
            window_size: None,
            pixel_size: None,
            screen_size: None,
            mouse_position_cache: None,
            window_alive: true,
            full_screen: false,
            vsync: false,
            title: "",
            running: true,
        }
    }
    fn init(&mut self) {
        self.new_key_state_map = Some(vec![false; 256]);
        self.old_key_state_map = Some(vec![false; 256]);
        self.new_mouse_state_map = Some(vec![false; 3]);
        self.old_mouse_state_map = Some(vec![false; 3]);
        self.key_map = Some(vec![HWButton::new(); 256]);
        self.mouse_map = Some(vec![HWButton::new(); 3]);
        self.mouse_position = Some(Vf2d::new(0.0, 0.0));
        self.view_position = Some(Vi2d::new(0, 0));
        self.window_size = Some(Vi2d::new(0, 0));
        self.screen_size = Some(Vi2d::new(0, 0));
        self.pixel_size = Some(Vi2d::new(0, 0));
        self.mouse_position_cache = Some(Vf2d::new(0.0, 0.0));
    }

    fn update_mouse(&mut self, mut x: i16, mut y: i16) {
        let mut x = x as i32;
        let mut y = y as i32;
        self.mouse_focus = true;
        x -= self.view_position.unwrap_or_default().x;
        y -= self.view_position.unwrap_or_default().y;
        let mut temp_mouse = Vf2d::from((
            (x as f32) / (self.window_size.unwrap_or_default().x - (self.view_position.unwrap_or_default().x * 2)) as f32 * (self.screen_size.unwrap_or_default().x as f32),
            (y as f32) / (self.window_size.unwrap_or_default().y - (self.view_position.unwrap_or_default().y * 2)) as f32 * (self.screen_size.unwrap_or_default().y as f32)
        ));
        if temp_mouse.x >= self.screen_size.unwrap_or_default().x as f32 {
            temp_mouse.x = (self.screen_size.unwrap_or_default().x - 1) as f32
        }
        if temp_mouse.y >= self.screen_size.unwrap_or_default().y as f32 {
            temp_mouse.y = (self.screen_size.unwrap_or_default().y - 1) as f32
        }
        if temp_mouse.x < 0.0 { temp_mouse.x = 0.0 }
        if temp_mouse.y < 0.0 { temp_mouse.y = 0.0 }
        temp_mouse.x = temp_mouse.x / self.pixel_size.unwrap_or_default().x as f32;
        temp_mouse.y = temp_mouse.y / self.pixel_size.unwrap_or_default().y as f32;
        self.mouse_position_cache = Some(temp_mouse);
    }
    fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_size = Some(Vi2d::from(((width as i32),
                                            (height as i32))));
    }
    fn update_mouse_wheel(&mut self, delta: i32) {
        self.mouse_wheel_delta_cache += delta;
    }
    fn update_mouse_focus(&mut self, b: bool) { self.mouse_focus = b }
    fn update_key_focus(&mut self, b: bool) { self.key_focus = b }
    fn update_key_state(&mut self, i: i32, b: bool) {
        self.new_key_state_map.as_mut().unwrap()[i as usize] = b;
    }
    fn update_mouse_state(&mut self, i: i32, b: bool) {
        self.new_mouse_state_map.as_mut().unwrap()[i as usize] = b;
    }
}

#[cfg(windows)]
impl PlatformWindows {
    pub fn new() -> PlatformWindows {
        PlatformWindows {
            hwnd: 0 as HWND,
            running: true,
        }
    }
    unsafe extern "system" fn handle_window_event(hw: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM)
                                                  -> LRESULT {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                DestroyWindow(hw);
                0
            }
            WM_MOUSEMOVE => {
                let x: i16 = (lparam & 0xFFFF) as i16;
                let y: i16 = ((lparam as u32 >> 16) & 0xFFFF) as i16;
                PLATFORM_DATA.update_mouse(x, y);
                0
            }
            WM_SIZE => {
                PLATFORM_DATA.update_window_size(
                    (lparam as u32) & 0xFFFF,
                    (lparam as u32 >> 16) & 0xFFFF);
                0
            }
            WM_MOUSEHWHEEL => {
                PLATFORM_DATA.update_mouse_wheel(
                    GET_WHEEL_DELTA_WPARAM(wparam) as i32);
                0
            }
            WM_MOUSELEAVE => {
                PLATFORM_DATA.update_mouse_focus(false);
                0
            }
            WM_SETFOCUS => {
                PLATFORM_DATA.update_key_focus(true);
                0
            }
            WM_KILLFOCUS => {
                PLATFORM_DATA.update_key_focus(false);
                0
            }
            WM_KEYDOWN => {
                PLATFORM_DATA.update_key_state(wparam as i32, true);
                0
            }
            WM_KEYUP => {
                PLATFORM_DATA.update_key_state(wparam as i32, false);
                0
            }
            WM_LBUTTONDOWN => {
                PLATFORM_DATA.update_mouse_state(0, true);
                0
            }
            WM_LBUTTONUP => {
                PLATFORM_DATA.update_mouse_state(0, false);
                0
            }
            WM_RBUTTONDOWN => {
                PLATFORM_DATA.update_mouse_state(1, true);
                0
            }
            WM_RBUTTONUP => {
                PLATFORM_DATA.update_mouse_state(1, false);
                0
            }
            WM_MBUTTONDOWN => {
                PLATFORM_DATA.update_mouse_state(2, true);
                0
            }
            WM_MBUTTONUP => {
                PLATFORM_DATA.update_mouse_state(2, false);
                0
            }
            //WM_CLOSE => {0}
            _ => {
                DefWindowProcA(hw, msg, wparam, lparam)
            }
        }
    }
}

//This PLATFORM_INSTANCE handle needs to be updated per platform
#[cfg(windows)]
static mut PLATFORM_INSTANCE: HWND = 0 as HWND;

#[cfg(windows)]
impl Platform for PlatformWindows {
    fn thread_cleanup(&self) -> Rcode {
        unsafe {
            //renderer.destroy_device();
            PostMessageA(self.hwnd, WM_DESTROY, 0, 0);
        }
        Rcode::Ok
    }

    fn create_window_pane(&mut self, window_pos: Vi2d,
                          mut window_size: Vi2d,
                          full_screen: bool) -> Rcode {
        unsafe {
            //let HInstance = GetModuleHandleA();
            let class_string = match WideCString::from_str_with_nul("OLC_PIXEL_GAME_ENGINE\0") {
                Ok(v) => v.to_string_lossy(),
                Err(e) => return Rcode::Fail,
            };
            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                hIcon: LoadIconW(NULL as HINSTANCE, IDI_APPLICATION),
                hCursor: LoadCursorW(NULL as HINSTANCE, IDC_ARROW),
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                hInstance: GetModuleHandleW(0 as LPCWSTR),
                lpfnWndProc: Some(PlatformWindows::handle_window_event),
                cbClsExtra: 0,
                cbWndExtra: 0,
                lpszMenuName: 0 as *const u16,
                hbrBackground: COLOR_WINDOW as HBRUSH,
                lpszClassName: class_string.as_ptr() as *const u16,
                hIconSm: LoadIconW(NULL as HINSTANCE, IDC_ARROW),
            };
            RegisterClassExW(&wc);

            let mut dwExStyle: DWORD = WS_EX_APPWINDOW | WS_EX_WINDOWEDGE;
            let mut dwStyle: DWORD = WS_CAPTION | WS_SYSMENU | WS_VISIBLE;
            let mut top_left = window_pos;
            if full_screen {
                dwExStyle = 0;
                dwStyle = WS_VISIBLE | WS_POPUP;
                let hmon = MonitorFromWindow(self.hwnd,
                                             MONITOR_DEFAULTTOPRIMARY);

                let mut mi: MONITORINFO = std::mem::uninitialized();
                GetMonitorInfoW(hmon, &mut mi);

                window_size = Vi2d::new(mi.rcMonitor.right,
                                        mi.rcMonitor.bottom);
                top_left.x = 0;
                top_left.y = 0;
            }
            let left = window_size.x;
            let bottom = window_size.y;
            let mut wnd_rect = RECT {
                left: 0,
                top: 0,
                right: left,
                bottom,
            };

            AdjustWindowRectEx(&mut wnd_rect, dwStyle, 0, dwExStyle);
            let width = &wnd_rect.right - &wnd_rect.left;
            let height = &wnd_rect.bottom - &wnd_rect.top;
            let try_create = || -> HWND{
                CreateWindowExW(dwExStyle,
                                class_string.as_ptr() as *const u16,
                                class_string.as_ptr() as *const u16,
                                dwStyle, top_left.x, top_left.y,
                                width, height,
                                0 as *mut HWND__,
                                0 as *mut HMENU__,
                                GetModuleHandleW(0 as LPCWSTR),
                                NULL)
            };
            let mut tries = 0;
            PLATFORM_DATA.window_size = Some(window_size);
            self.hwnd = try_create();
            let test_create = |hwnd: HWND| -> bool{
                let pid = hwnd as *const HWND as u64;
                if pid == 0 {
                    false
                } else {
                    true
                }
            };
            tries += 1;
            while tries < 5 && !test_create(self.hwnd) {
                self.hwnd = try_create();
                tries += 1;
            }
            if tries >= 5 { panic!("Could not create Window"); }
            PLATFORM_INSTANCE = self.hwnd;
            //this gets moved
            //ShowWindow(self.hwnd, 1);
            //UpdateWindow(self.hwnd);
        }
        Rcode::Ok
    }

    fn set_window_title(&self, mut title: String) -> Rcode {
        //title.push('\0');
        let title_string = CString::new(title).expect("");
        unsafe {
            SetWindowTextW(PLATFORM_INSTANCE, title_string.as_ptr() as LPCWSTR);
        }
        Rcode::Ok
    }

    fn start_system_event_loop(&self) -> Rcode {
        unsafe {
            //We have to initialize the data first so that Rust feels comfortable
            let mut lpMsg: MSG = std::mem::uninitialized();
            loop {
                if GetMessageW(&mut lpMsg, NULL as HWND, 0, 0) > 0 {
                    TranslateMessage(&mut lpMsg);
                    DispatchMessageW(&mut lpMsg);
                } else {
                    return Rcode::Fail;
                }
                if !PLATFORM_DATA.running { return Rcode::Fail; }
            }
        }
        Rcode::Ok
    }

    fn handle_system_event(&self) -> Rcode {
        Rcode::Fail
    }
}

pub struct OLCEngine {
    pub app_name: String,
    pub is_focused: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub pixels_w: u32,
    pub pixels_h: u32,
    pub fps: u32,
    inv_screen_size: Vf2d,
    draw_target: u32,
    full_screen: bool,
    vsync: bool,
    layers: Vec<LayerDesc>,
    game_timer: SystemTime,
    mouse_position: Vi2d,
    font_decal: Decal,
}


impl OLCEngine {
    pub fn empty() -> Self {
        let mut engine = OLCEngine {
            app_name: String::from(""),
            is_focused: true,
            window_width: 0,
            window_height: 0,
            pixels_w: 0,
            pixels_h: 0,
            pixel_width: 0,
            pixel_height: 0,
            inv_screen_size: Vf2d::new(0.0, 0.0),
            fps: 0,
            full_screen: false,
            vsync: false,
            layers: vec![],
            game_timer: std::time::SystemTime::now(),
            draw_target: 0,
            mouse_position: Vi2d::from((0, 0)),
            font_decal: Decal::new(),
        };
        engine
    }
    pub fn init(app_name: &str,
               screen_width: u32,
               screen_height: u32,
               pixel_width: u32,
               pixel_height: u32,
               full_screen: bool,
               vsync: bool){
        let tex_size_w = screen_width / pixel_width;
        let tex_size_h = screen_height / pixel_height;
        let inv_screen_size = Vf2d::from((
            (1.0 / tex_size_w as f32),
            (1.0 / tex_size_h as f32)
        ));
        unsafe {
            let mut engine = PGE.get_or_init(|| {
                OLCEngine {
                    app_name: String::from(app_name),
                    is_focused: true,
                    window_width: screen_width,
                    window_height: screen_height,
                    pixels_w: tex_size_w,
                    pixels_h: tex_size_h,
                    pixel_width: pixel_width,
                    pixel_height: pixel_height,
                    inv_screen_size: inv_screen_size,
                    fps: 0,
                    full_screen: full_screen,
                    vsync: vsync,
                    layers: vec![],
                    game_timer: std::time::SystemTime::now(),
                    draw_target: 0,
                    mouse_position: Vi2d::new(0, 0),
                    font_decal: Decal::new(),
                }
            });
        }
    }
}

pub fn is_focused() -> bool {
    unsafe { PLATFORM_DATA.key_focus }
}

pub fn get_key(k: Key) -> HWButton {
    unsafe {
        PLATFORM_DATA.key_map.as_mut().unwrap()[Keys::map_keys(k) as usize]
    }
}

pub fn set_key(i: usize, k: HWButton) {
    unsafe {
        PLATFORM_DATA.key_map.as_mut().unwrap()[i] = k
    }
}

pub fn get_mouse(b: i32) -> HWButton {
    unsafe {
        PLATFORM_DATA.mouse_map.as_ref().unwrap()[b as usize]
    }
}
pub fn set_mouse(i: usize, k: HWButton) {
    unsafe {
        PLATFORM_DATA.mouse_map.as_mut().unwrap()[i] = k
    }
}

pub fn mouse_x() -> f32 {
    unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().x }
}

pub fn mouse_y() -> f32 {
    unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default().y }
}

pub fn mouse_wheel() -> i32 { 0 }

//pub fn get_window_mouse() -> Vi2d { Vi2d }

pub fn mouse_pos() -> Vf2d {
    unsafe { PLATFORM_DATA.mouse_position.unwrap_or_default() }
}

//Utility
pub fn screen_width() -> i32 {
    unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().x }
}

pub fn screen_height() -> i32 {
    unsafe { PLATFORM_DATA.screen_size.unwrap_or_default().y }
}


pub fn get_draw_target_width() -> i32 {
    get_draw_target_ref().sprite.width as i32
}

pub fn get_draw_target_height() -> i32 {
    get_draw_target_ref().sprite.height as i32
}

pub fn set_screen_size(w: i32, h: i32) {}

//This is overriden by drawing onto Layers.
//We may want to do a "DrawTarget" Trait.
//pub fn set_draw_target(target: &mut Sprite) {}
//pub fn get_draw_target() -> Sprite { Sprite }

pub fn get_fps() -> i32 { 0 }

pub fn get_elapsed_time() -> f32 { 0.0 }

pub fn get_window_size() -> Vi2d {
    unsafe {
        PLATFORM_DATA.window_size.unwrap()
    }
}

pub fn get_screen_pixel_size() -> Vi2d {
    let local_pge_lock = unsafe{ PGE.get().unwrap()};
    Vi2d::new(local_pge_lock.window_width as i32, local_pge_lock.window_height
        as i32)
}

pub fn set_draw_target(layer_id: u32) {
    let mut local_pge_lock = unsafe{PGE.get_mut().unwrap()};
    (*local_pge_lock).draw_target = layer_id;
    set_layer_update(layer_id, true);
}
pub fn reset_draw_target() {
    let mut local_pge_lock = unsafe{PGE.get_mut().unwrap()};
    //set back to background layer
    local_pge_lock.draw_target = local_pge_lock.layers[0].id;
}

pub fn get_draw_target<'a>() -> &'a mut LayerDesc {
    get_layer(unsafe{PGE.get().unwrap()}.draw_target).unwrap()
}

pub fn get_draw_target_ref<'a>() -> &'a LayerDesc {
    get_layer_ref(unsafe{PGE.get().unwrap()}.draw_target).unwrap()
}

pub fn set_layer_visible(layer_id: u32, b: bool) {
    get_layer(layer_id).unwrap().shown = b;
}

pub fn set_layer_update(layer_id: u32, b: bool) {
    get_layer(layer_id).unwrap().update = b;
}

pub fn set_layer_offset(layer: u8, x: f32, y: f32) {}

pub fn set_layer_scale(layer: u8, x: f32, y: f32) {}

pub fn set_layer_tint(layer: u8, tint: Pixel) {}

//We'll come back to this
//pub fn set_layer_custom_render_function

pub fn get_layer_ref<'a>( layer_id: u32) -> Option<&'a LayerDesc> {
    let layer_iter = unsafe{PGE.get().unwrap()}.layers.iter();
    for layer in layer_iter {
        if layer.id == layer_id {
            return Some(layer);
        }
    }
    None
}

pub fn get_layer<'a>(layer_id: u32) -> Option<&'a mut LayerDesc> {
    let layer_iter = unsafe{PGE.get_mut().unwrap()}.layers.iter_mut();
    for layer in layer_iter {
        if layer.id == layer_id {
            return Some(layer);
        }
    }
    None
}

pub fn add_layer() -> u32 {
    let mut local_pge_lock = unsafe{PGE.get_mut().unwrap()};
    let layer = LayerDesc::new(local_pge_lock.pixels_w,
                               local_pge_lock.pixels_h);
    let r_id = layer.id;
    local_pge_lock.layers.push(layer);
    r_id
}

pub fn load_sprite(path: &str) -> Sprite {
    Sprite::load_from_file::<BMPLoader>(path)
}

fn push_decal_instance(di: DecalInstance) {
    get_draw_target().vec_decal_instance.push(di);
}

pub fn set_pixel_mode(m: PixelMode) {}

pub fn get_pixel_mode() -> PixelMode { PixelMode::Normal }

pub fn set_pixel_blend(blend: f32) {}

//DRAW ROUTINES
pub fn draw(x: i32, y: i32, p: Pixel) {
    get_draw_target().sprite.set_pixel(x as u32, y as u32, p);
}

pub fn draw_line(pos1: Vi2d, pos2: Vi2d, p: Pixel, pattern: u32) {
    draw_line_xy(pos1.x, pos1.y, pos2.x, pos2.y, p);
}

pub fn draw_line_xy(mut x1: i32, mut y1: i32,
                    mut x2: i32, mut y2: i32, p: Pixel) {
    let (mut x, mut y, mut dx, mut dy, mut dx1,
        mut dy1, mut px, mut py, mut xe, mut ye, mut i) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    dx = x2 - x1;
    dy = y2 - y1;

    if dx == 0 {
        if y2 < y1 { std::mem::swap(&mut y1, &mut y2); }
        for y in y1..=y2 {
            draw(x1, y, p);
        }
        return;
    }

    if dy == 0 {
        if x2 < x1 { std::mem::swap(&mut x1, &mut x2); }
        for x in x1..=x2 {
            draw(x, y1, p);
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

        draw(x, y, p);

        for i in x..xe {
            x = x + 1;
            if px < 0 {
                px = px + 2 * dy1;
            } else {
                if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                    y = y + 1;
                } else {
                    y = y - 1;
                }
                px = px + 2 * (dy1 - dx1);
            }
            draw(x, y, p);
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
        draw(x, y, p);

        for i in y..ye {
            y = y + 1;
            if py <= 0 {
                py = py + 2 * dx1;
            } else {
                if (dx < 0 && dy < 0) || (dx > 0 && dy > 0) {
                    x = x + 1
                } else {
                    x = x - 1;
                }
                py = py + 2 * (dx1 - dy1);
            }
            draw(x, y, p);
        }
    }
}

pub fn draw_circle(pos: Vi2d, r: i32, p: Pixel, mask: u32) {
    draw_circle_xy(pos.x, pos.y, r, p, mask);
}

pub fn draw_circle_xy(x: i32, y: i32, r: i32,
                      p: Pixel, mask: u32) {
    if r < 0 || x < -r || y < -r || x - get_draw_target_ref().sprite.width as i32 > r
        || y - get_draw_target_ref().sprite.height as i32 > r {
        return;
    }
    if r > 0 {
        let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
        while y0 >= x0 {
            if mask & 0x01 == 0x01 { draw(x + x0, y - y0, p) };
            if mask & 0x04 == 0x04 { draw(x + y0, y + x0, p) };
            if mask & 0x10 == 0x10 { draw(x - x0, y + y0, p) };
            if mask & 0x40 == 0x40 { draw(x - y0, y - x0, p) };
            if x0 != 0 && x0 != y0 {
                if mask & 0x02 == 0x02 { draw(x + y0, y - x0, p) };
                if mask & 0x08 == 0x08 { draw(x + x0, y + y0, p) };
                if mask & 0x20 == 0x20 { draw(x - y0, y + x0, p) };
                if mask & 0x80 == 0x80 { draw(x - x0, y - y0, p) };
            }
            if d < 0 {
                x0 += 1;
                d += 4 * x0 + 6;
            } else {
                x0 += 1;
                y0 -= 1;
                d += 4 * (x0 - y0) + 10;
            }
        }
    } else {
        draw(x, y, p);
    }
}

pub fn fill_circle(pos: Vf2d, r: i32, p: Pixel) {
    fill_circle_xy(pos.x as i32, pos.y as i32, r, p);
}

pub fn fill_circle_xy(mut x: i32, mut y: i32, r: i32, p: Pixel) {
    if r < 0 || x < -r || y < -r || x - get_draw_target_ref().sprite.width as i32 > r || y
        - get_draw_target_ref().sprite.height as i32 > r {
        return;
    }

    if r > 0 {
        let (mut x0, mut y0, mut d) = (0, r, 3 - 2 * r);
        let mut drawline = |sx: i32, ex: i32, y: i32| {
            for x in sx..=ex {
                draw(x, y, p);
            }
        };

        while y0 >= x0 {
            drawline(x - y0, x + y0, y - x0);
            if x0 > 0 { drawline(x - y0, x + y0, y + x0); }

            if d < 0 {
                x0 += 1;
                d += 4 * x0 + 6;
            } else {
                if x0 != y0 {
                    drawline(x - x0, x + x0, y - y0);
                    drawline(x - x0, x + x0, y + y0);
                }
                x0 += 1;
                y0 -= 1;
                d += 4 * (x0 - y0) + 10;
            }
        }
    } else {
        draw(x, y, p);
    }
}
pub fn draw_rect(pos: Vi2d, size: Vi2d, p: Pixel) {
    draw_rect_xy(pos.x, pos.y, size.x, size.y, p);
}
pub fn draw_rect_xy(x: i32, y: i32, w: i32, h: i32, p: Pixel) {
    draw_line_xy(x, y, x + w, y, p);
    draw_line_xy(x + w, y, x + w, y + h, p);
    draw_line_xy(x + w, y + h, x, y + h, p);
    draw_line_xy(x, y + h, x, y, p);
}

pub fn draw_trangle(pos1: Vf2d, pos2: Vf2d,
                    pos3: Vf2d, p: Pixel) {
    draw_triangle_xy(pos1.x as i32, pos1.y as i32, pos2.x as i32, pos2.y as i32,
                          pos3.x as i32, pos3.y as i32, p);
}

pub fn draw_triangle_xy(x1: i32, y1: i32, x2: i32,
                        y2: i32, x3: i32, y3: i32, p: Pixel) {
    draw_line_xy(x1, y1, x2, y2, p);
    draw_line_xy(x2, y2, x3, y3, p);
    draw_line_xy(x3, y3, x1, y1, p);
}


pub fn fill_triangle(mut pos1: Vf2d, mut pos2: Vf2d,
                    mut pos3: Vf2d, p: Pixel) {
    //Sort the points so that y1 <= y2 <= y3
    if pos2.y < pos1.y {std::mem::swap(&mut pos2, &mut pos1);}
    if pos3.y < pos1.y {std::mem::swap(&mut pos3, &mut pos1);}
    if pos3.y < pos2.y {std::mem::swap(&mut pos3, &mut pos2);}
    //This takes two vectors and a y position and returns the x coordinate
    let interpolate = |l: &Vf2d, r: &Vf2d, y: f32| -> f32{
        let c = (r.x-l.x)/(r.y-l.y);
        (c * (y - l.y)) + l.x
    };

    for y in pos1.y.floor() as i32..pos3.y.floor() as i32{
        if y <= pos2.y.floor() as i32{
            for x in interpolate(&pos1, &pos3, y as f32) as i32..
                interpolate(&pos1, &pos2,y as f32) as i32{
                draw(x as i32, y, p);
            }
        } else {
            for x in interpolate(&pos1, &pos3, y as f32) as i32..
                interpolate(&pos2, &pos3,y as f32) as i32{
                draw(x as i32, y, p);
            }
        }
    }

}

pub fn clear(p: Pixel) {
    let pixels = get_draw_target_height() * get_draw_target_width();

    let mut m = &mut get_draw_target().sprite.col_data;
    for i in 0..pixels {
        m[i as usize] = p;
    }
}

fn construct_font_sheet() -> Decal {
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
    let mut i = 0;
    for c in data.chars() {
        data_chars[i] = c as u8;
        i += 1;
    }
    for b in (0..1024).step_by(4) {
        let sym1: u32 = (data_chars[b + 0] as u32) - 48;
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
    Decal::create(Some(font_sprite))
}

pub fn draw_decal(pos: Vf2d, decal: &Decal){
    draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0,1.0), Pixel::WHITE);
}

pub fn draw_decal_with_scale(pos: Vf2d, decal: &Decal,
                  scale: Vf2d){
    draw_decal_with_scale_and_tint(pos, decal, scale, Pixel::WHITE);
}
pub fn draw_decal_with_tint(pos: Vf2d, decal: &Decal, tint: Pixel){
    draw_decal_with_scale_and_tint(pos, decal, Vf2d::new(1.0,1.0), tint);
}

pub fn draw_decal_with_scale_and_tint(pos: Vf2d, decal: &Decal,
                  scale: Vf2d, tint: Pixel) {
    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let screen_space_pos = Vf2d::from((
        (pos.x * local_pge_lock.inv_screen_size.x) * 2.0 - 1.0,
        ((pos.y * local_pge_lock.inv_screen_size.y) * 2.0 - 1.0) * -1.0
    ));
    let screen_space_dim = Vf2d::from((
        screen_space_pos.x + (2.0 * (decal.get().sprite.width as f32) *
            local_pge_lock.inv_screen_size.x),
        screen_space_pos.y - (2.0 * (decal.get().sprite.height as f32) *
            local_pge_lock.inv_screen_size.y)
    ));
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());
    di.tint[0] = tint;
    di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
    di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
    di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
    di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));
    push_decal_instance(di);
    //self.get_draw_target().vec_decal_instance.push(di);
}

pub fn draw_partial_decal(pos: Vf2d, decal: &Decal,
                          source_pos: Vf2d, source_size: Vf2d, scale: Vf2d,
                          tint: Pixel) {
    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let screen_space_pos = Vf2d::from((
        (pos.x * local_pge_lock.inv_screen_size.x) * 2.0 - 1.0,
        ((pos.y * local_pge_lock.inv_screen_size.y) * 2.0 - 1.0) * -1.0
    ));
    let screen_space_dim = Vf2d::from((
        screen_space_pos.x + (2.0 * (source_size.x as f32) * local_pge_lock.inv_screen_size.x) *
            scale.x,
        screen_space_pos.y - (2.0 * (source_size.y as f32) * local_pge_lock
            .inv_screen_size.y) * scale.y
    ));
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());
    di.tint[0] = tint;

    di.pos[0] = Vf2d::from((screen_space_pos.x, screen_space_pos.y));
    di.pos[1] = Vf2d::from((screen_space_pos.x, screen_space_dim.y));
    di.pos[2] = Vf2d::from((screen_space_dim.x, screen_space_dim.y));
    di.pos[3] = Vf2d::from((screen_space_dim.x, screen_space_pos.y));

    let uvtl = Vf2d::from((
        source_pos.x * decal.get().uv_scale.x,
        source_pos.y * decal.get().uv_scale.y
    ));

    let uvbr = Vf2d::from((
        uvtl.x + (source_size.x * decal.get().uv_scale.x),
        uvtl.y + (source_size.y * decal.get().uv_scale.y)
    ));

    di.uv[0] = Vf2d::from((uvtl.x, uvtl.y));
    di.uv[1] = Vf2d::from((uvtl.x, uvbr.y));
    di.uv[2] = Vf2d::from((uvbr.x, uvbr.y));
    di.uv[3] = Vf2d::from((uvbr.x, uvtl.y));
    push_decal_instance(di);
}

pub fn draw_rotated_decal(pos: Vf2d, decal: &Decal,
                          angle: f32, center: Vf2d, scale: Vf2d, tint: Pixel) {
    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let mut di = DecalInstance::new();
    let d_ref = decal.get();
    di.decal = Some(decal.get());
    di.tint[0] = tint;
    di.pos[0] = Vf2d::new(0.0 - center.x * scale.x,
                          0.0 - center.y * scale.y);
    di.pos[1] = Vf2d::new(0.0 - center.x * scale.x,
                          d_ref.sprite.height as f32 - center.y * scale.y);
    di.pos[2] = Vf2d::new(d_ref.sprite.width as f32 - center.x * scale.x,
                          d_ref.sprite.height as f32 - center.y * scale.y);
    di.pos[3] = Vf2d::new(d_ref.sprite.width as f32 - center.x * scale.x,
                          0.0 - center.y * scale.y);
    let (c, s) = (angle.cos(), angle.sin());
    for i in 0..4 {
        di.pos[i] = Vf2d::new(
            di.pos[1].x * c - di.pos[i].y * s,
            di.pos[i].x * s + di.pos[i].y * c);
        di.pos[i] = Vf2d::new(di.pos[i].x * local_pge_lock.inv_screen_size.x * 2.0 - 1.0,
                              di.pos[i].y * local_pge_lock.inv_screen_size.y * 2.0 - 1.0);
        di.pos[i].y += -1.0;
    }
    push_decal_instance(di);
}


pub fn draw_warped_decal(decal: &Decal, pos: Vec<Vf2d>) {
    draw_warped_decal_with_tint(decal, pos, Pixel::WHITE);
}

pub fn draw_warped_decal_with_tint(decal: &Decal, pos: Vec<Vf2d>, tint: Pixel) {
    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());
    di.tint[0] = tint;
    let mut center = Vf2d::new(0.0, 0.0);
    let mut rd: f32 = ((pos[2].x - pos[0].x) * (pos[3].y - pos[1].y) -
        (pos[3].x - pos[1].x) * (pos[2].y - pos[0].y));
    if rd != 0.0 {
        rd = 1.0 / rd;
        let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y) -
            (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x)) * rd;
        let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y) -
            (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x)) * rd;
        if !(rn < 0.0 || rn > 1.0 || sn < 0.0 || sn > 1.0) {
            let i = pos[2] - pos[0];
            center = pos[0] + Vf2d::new(rn * i.x, rn * i.y);
        }
        let mut d: [f32; 4] = [0.0; 4];
        for i in 0..4 {
            d[i] = (pos[i] - center).mag();
        }
        for i in 0..4 {
            let q = if d[i] == 0.0 { 1.0 } else { (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3] };
            di.uv[i].x *= q;
            di.uv[i].y *= q;
            di.w[i] *= q;
            di.pos[i] = Vf2d::new((pos[i].x * local_pge_lock.inv_screen_size.x) * 2.0 - 1.0,
                                  ((pos[i].y * local_pge_lock.inv_screen_size.y) * 2.0 - 1.0) *
                                      -1.0);
        }
        push_decal_instance(di);
    }
}

pub fn draw_partial_warped_decal(decal: &Decal, pos: Vec<Vf2d>,
                                 source_pos: Vf2d, source_size: Vf2d, tint: Pixel){

    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());
    di.tint[0] = tint;
    let mut center = Vf2d::new(0.0,0.0);
    let mut rd: f32 = ((pos[2].x - pos[0].x) * (pos[3].y - pos[1].y) - (pos[3].x -
        pos[1].x) * (pos[2].y - pos[0].y));
    if rd != 0.0 {
        let uvtl = Vf2d::new(source_pos.x * decal.get().uv_scale.x,
                             source_pos.y * decal.get().uv_scale.y);

        let uvbr = Vf2d::new(uvtl.x + (source_size.x * decal.get().uv_scale.x),
                             uvtl.y + (source_size.y * decal.get().uv_scale.y));

        di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
        di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
        di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
        di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
        rd = 1.0 / rd;
        let rn: f32 = ((pos[3].x - pos[1].x) * (pos[0].y - pos[1].y) - (pos[3].y - pos[1].y) * (pos[0].x - pos[1].x)) * rd;
        let sn: f32 = ((pos[2].x - pos[0].x) * (pos[0].y - pos[1].y) - (pos[2].y - pos[0].y) * (pos[0].x - pos[1].x)) * rd;
        if !(rn < 0.0 || rn > 1.0 || sn < 0.0 || sn > 1.0) {
            let i = pos[2] - pos[0];
            center = Vf2d::new(pos[0].x + rn, pos[0].y + rn) * i;
        }
        let mut d: [f32; 4] = [0.0; 4];
        for i in 0..4 {
            d[i] = (pos[i] - center).mag();
        }
        for i in 0..4 {
            let q = if d[i] == 0.0 { 1.0 } else { (d[i] + d[(i + 2) & 3]) / d[(i + 2) & 3] };
            di.uv[i].x *= q;
            di.uv[i].y *= q;
            di.w[i] *= q;
            di.pos[i] = Vf2d::new((pos[i].x * local_pge_lock.inv_screen_size.x) * 2.0 - 1.0,
                                  ((pos[i].y * local_pge_lock.inv_screen_size.y) *
                                      2.0 - 1.0) * -1.0);
        }
        push_decal_instance(di);
    }
}

pub fn draw_explicit_decal(decal: &Decal,
                           pos: Vec<Vf2d>, uv: Vec<Vf2d>, col: Vec<Pixel>) {
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());

    let local_pge_lock = unsafe{PGE.get().unwrap()};
    unsafe {
        for i in 0..4 {
            di.pos[i] = Vf2d::from((
                (pos[i].x * local_pge_lock.inv_screen_size.x) * 2.0 - 1.0,
                (pos[i].y * local_pge_lock.inv_screen_size.y) * 2.0 - 1.0));
            di.uv[i] = uv[i];
            di.tint[i] = col[i];
        }
    }
    push_decal_instance(di);
}

pub fn fill_rect_decal(pos: Vf2d, size: Vf2d, col: Pixel) {
    let points = vec![pos, Vf2d::new(pos.x, pos.y + size.y),
                      pos + size, Vf2d::new(pos.x + size.x, pos.y)];
    let uvs = vec![Vf2d::new(0.0, 0.0), Vf2d::new(0.0, 0.0),
                   Vf2d::new(0.0, 0.0), Vf2d::new(0.0, 0.0)];
    let cols = vec![col, col, col, col];
    draw_explicit_decal(&Decal::new(), points, uvs, cols);
}

pub fn gradient_fill_rect_decal(pos: Vf2d, size: Vf2d,
                                colTL: Pixel, colBL: Pixel,
                                colTR: Pixel, colBR: Pixel) {
    let points = vec![pos, Vf2d::new(pos.x, pos.y + size.y),
                      pos + size, Vf2d::new(pos.x + size.x, pos.y)];
    let uvs = vec![Vf2d::new(0.0, 0.0), Vf2d::new(0.0, 0.0),
                   Vf2d::new(0.0, 0.0), Vf2d::new(0.0, 0.0)];
    let cols = vec![colTL, colBL, colBR, colTR];
    draw_explicit_decal(&Decal::new(), points, uvs, cols);
}

pub fn draw_partial_rotated_decal(pos: Vf2d, decal: &Decal,
                                  angle: f32, center: Vf2d, source_pos: Vf2d,
                                  source_size: Vf2d, scale: Vf2d, tint: Pixel) {
    let local_pge_lock = unsafe{PGE.get().unwrap()};
    let mut di = DecalInstance::new();
    di.decal = Some(decal.get());
    di.tint[0] = tint;
    di.pos[0] = (Vf2d::new(0.0, 0.0) - center) * scale;
    di.pos[1] = (Vf2d::new(0.0, source_size.y) - center) * scale;
    di.pos[2] = (Vf2d::new(source_size.x, source_size.y) - center) * scale;
    di.pos[3] = (Vf2d::new(source_size.x, 0.0) - center) * scale;
    let (c, s) = (angle.cos(), angle.sin());
    for i in 0..4 {
        di.pos[i] = Vf2d::new(
            di.pos[1].x * c - di.pos[i].y * s,
            di.pos[i].x * s + di.pos[i].y * c);
        di.pos[i] = Vf2d::new(di.pos[i].x * local_pge_lock.inv_screen_size
            .x * 2.0 - 1.0,
                              di.pos[i].y * local_pge_lock.inv_screen_size.y * 2.0 - 1.0);
        di.pos[i].y += -1.0;
    }
    let uvtl = Vf2d::new(source_pos.x * decal.get().uv_scale.x,
                         source_pos.y * decal.get().uv_scale.y);

    let uvbr = Vf2d::new(uvtl.x + (source_size.x * decal.get().uv_scale.x),
                         uvtl.y + (source_size.y * decal.get().uv_scale.y));

    di.uv[0] = Vf2d::new(uvtl.x, uvtl.y);
    di.uv[1] = Vf2d::new(uvtl.x, uvbr.y);
    di.uv[2] = Vf2d::new(uvbr.x, uvbr.y);
    di.uv[3] = Vf2d::new(uvbr.x, uvtl.y);
    push_decal_instance(di);
}

pub fn draw_string_decal(pos: Vf2d, text: &str, col: Pixel,
                         scale: Vf2d) {
    //self.draw_decal(pos, &FONT_DECAL, scale, Pixel::WHITE);
    let mut spos = Vf2d::new(0.0, 0.0);
    for c in text.chars() {
        if c == '\n' {
            spos.x = 0.0;
            spos.y += 8.0 * scale.y;
        } else {
            let ox = (c as u8 - 32) % 16;
            let oy = (c as u8 - 32) / 16;
            draw_partial_decal(pos + spos, &FONT_DECAL,
                                    Vf2d::new(ox as f32 * 8.0, oy as f32 * 8.0),
                                    Vf2d::new(8.0, 8.0), scale, col);
            spos.x += 8.0 * scale.x;
        }
    }
}

pub fn get_text_size(s: String) -> Vi2d {
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

pub trait Olc {
    fn on_engine_start(&mut self) -> bool;

    fn on_engine_update(&mut self, elapsedTime: f32)-> bool;

    fn on_engine_destroy(&mut self)-> bool;
}

pub trait App: Olc {
    fn construct(
        &self,
        app_name: &'static str,
        screen_width: u32,
        screen_height: u32,
        pixel_width: u32,
        pixel_height: u32,
        full_screen: bool,
        vsync: bool,
    ) -> bool {
        //Set the olc object to be used in this crate
        unsafe {
            PLATFORM_DATA.init();
            PLATFORM_DATA.window_size = Some(
                Vi2d::from((screen_width as i32, screen_height as i32)));
            PLATFORM_DATA.full_screen = full_screen;
            PLATFORM_DATA.title = app_name;
            PLATFORM_DATA.pixel_size = Some(Vi2d::new(pixel_width as i32,
                                                      pixel_height as i32));
        };
        OLCEngine::init(
            app_name,
            screen_width,
            screen_height,
            pixel_width,
            pixel_height,
            full_screen,
            vsync,
        );
        true
    }

    fn start(&mut self ) -> Rcode {
        let local_pge_lock = unsafe{PGE.get_mut().unwrap()};
        //Hardcoded to Windows for now. Will change in the future.
        #[cfg(windows)] let mut game_platform = PlatformWindows::new();

        let (tx, rx) = channel();
        //Move all WINDOW functions to another thread
        let t = thread::spawn(move || {
            let mut platform = PlatformWindows::new();
            platform.thread_startup();
            unsafe {
                platform.create_window_pane(Vi2d { x: 50, y: 50 },
                                            PLATFORM_DATA.window_size.unwrap(),
                                            PLATFORM_DATA.full_screen);
            }
            //Let the main thread know the window is created
            tx.send(true);
            unsafe {
                platform.set_window_title(PLATFORM_DATA.title.to_string());
            }
            if let Rcode::Fail = platform.start_system_event_loop() {
                println!("End Window Loop");
                //Let the main thread know that the window has been destroyed
                tx.send(true);
            }
            Rcode::Ok
        });
        //Block waiting for window to be made
        rx.recv().unwrap_or_default();

        //PrepareEngine
        let mut renderer = Renderer::new();
        renderer.create_graphics(local_pge_lock.full_screen,
                                 local_pge_lock.vsync,
                                 Vi2d { x: 0, y: 0 },
                                 Vi2d {
                                     x: local_pge_lock.window_width as i32,
                                     y: local_pge_lock.window_height as i32
                                 });

        //Create Primary Layer "0"
        let base_layer_id = add_layer();
        set_draw_target(base_layer_id);


        let mut frame_timer: f32 = 0.0;
        let mut frame_count: i32 = 0;
        let mut last_fps: i32 = 0;
        let mut game_timer = std::time::SystemTime::now();
        Renderer::update_texture(get_draw_target_ref().id,
                                 &get_draw_target_ref().sprite);

        //game_engine.construct_font_sheet();
        let mut end_loop = !self.on_engine_start();
        while !end_loop {
            //Check at the beginning of every loop whether the window is
            // still alive.
            end_loop = rx.try_recv().unwrap_or_default();
            let elapsed_time = game_timer.elapsed().unwrap().as_secs_f32();
            game_timer = std::time::SystemTime::now();

            //TODO: CHECK INPUTS
            unsafe {
                let hw_func = |keys: &mut Vec<HWButton>,
                               keys_old: &mut Vec<bool>,
                               keys_new: &mut Vec<bool>,
                               size: usize| {
                    for i in 0..size as usize {
                        keys[i].pressed = false;
                        keys[i].released = false;
                        if keys_new[i] != keys_old[i] {
                            if keys_new[i] {
                                keys[i].pressed = !keys[i].held;
                                keys[i].held = true;
                            } else {
                                keys[i].released = true;
                                keys[i].held = false;
                            }
                        }
                        keys_old[i] = keys_new[i];
                    }
                };

                hw_func(PLATFORM_DATA.key_map.as_mut().unwrap(),
                        PLATFORM_DATA.old_key_state_map.as_mut().unwrap(),
                        PLATFORM_DATA.new_key_state_map.as_mut().unwrap(),
                        256
                );
                hw_func(PLATFORM_DATA.mouse_map.as_mut().unwrap(),
                        PLATFORM_DATA.old_mouse_state_map.as_mut().unwrap(),
                        PLATFORM_DATA.new_mouse_state_map.as_mut().unwrap(),
                        3
                );

                PLATFORM_DATA.mouse_position = Some(Vf2d::from((
                    PLATFORM_DATA.mouse_position_cache.unwrap().x as f32,
                    PLATFORM_DATA.mouse_position_cache.unwrap().y as f32)));
                PLATFORM_DATA.mouse_wheel_delta = PLATFORM_DATA.mouse_wheel_delta_cache;
                PLATFORM_DATA.mouse_wheel_delta_cache = 0;
            }
            if !self.on_engine_update(elapsed_time) {
                end_loop = true;
            }
            unsafe {
                Renderer::update_viewport(Vi2d { x: 0, y: 0 },
                                          Vi2d {
                                              x: PLATFORM_DATA.window_size.unwrap().x,
                                              y: PLATFORM_DATA.window_size.unwrap().y
                                          });
            }
            Renderer::clear_buffer(Pixel::rgba(0, 0, 0, 255), true);
            //always draw the background
            local_pge_lock.layers[0].update = true;
            local_pge_lock.layers[0].shown = true;
            Renderer::prepare_drawing();

            let mut layer_iter = local_pge_lock.layers.iter_mut();

            for layer in layer_iter {
                if layer.shown {
                    if let None = layer.func_hook {
                        Renderer::apply_texture(layer.id);
                        if layer.update {
                            Renderer::update_texture(layer.id,
                                                     &layer.sprite);
                            set_layer_update(layer.id, false);
                        }
                        Renderer::draw_layer_quad(layer.offset,
                                                  layer.scale,
                                                  layer.tint);
                        if !layer.vec_decal_instance.is_empty() {
                            let layer_decals = layer.vec_decal_instance.iter_mut();
                            for decal in layer_decals {
                                Renderer::draw_decal_quad(decal);
                            }
                            layer.vec_decal_instance.clear();
                        }
                    } else {
                        //Run the custom function hook
                        (layer.func_hook.unwrap())(layer);
                    }
                }
            }
            renderer.display_frame();
            // Update Title Bar
            frame_timer += elapsed_time;
            frame_count += 1;
            if frame_timer >= 1.0 {
                last_fps = frame_count;
                frame_timer -= 1.0;
                let sTitle = String::from(
                    local_pge_lock.app_name.to_string() + " - FPS: " + &*frame_count.to_string());
                unsafe { game_platform.set_window_title(sTitle) };
                frame_count = 0;
            }
        }
        println!("Loop Ended");
        unsafe { PLATFORM_DATA.running = false; }
        t.join().unwrap()?;
        self.on_engine_destroy();
        Rcode::Ok
    }
}

impl<T: Olc> App for T {}

impl<T> V2d<T> {
    pub fn new(x: T, y: T) -> Self { Self { x, y } }
}

impl Vi2d {
    /// Returns magnitude (or length) of a vector.
    pub fn mag(&self) -> i32 { (self.mag2() as f32).sqrt() as i32 }

    /// Returns magnitude squared.
    pub fn mag2(&self) -> i32 { self.x * self.x + self.y * self.y }

    /// Returns vector norm.
    pub fn norm(&self) -> Self {
        let r = 1 / self.mag();
        Self { x: self.x * r, y: self.y * r }
    }

    /// Returns perpendicular vector.
    pub fn perp(&self) -> Self { Self { x: -self.y, y: self.x } }

    /// Returns dot product of two vectors.
    pub fn dot(&self, rhs: Vi2d) -> i32 { self.x * rhs.x + self.y * rhs.y }

    /// Returns cross product of two vectors.
    pub fn cross(&self, rhs: Vi2d) -> i32 { self.x * rhs.y - self.y * rhs.x }
}

impl Vf2d {
    /// Returns magnitude (or length) of a vector.
    pub fn mag(&self) -> f32 { self.mag2().sqrt() }

    /// Returns magnitude squared.
    pub fn mag2(&self) -> f32 { self.x * self.x + self.y * self.y }

    /// Returns vector norm.
    pub fn norm(&self) -> Self {
        let r = 1.0 / self.mag();
        Self { x: self.x * r, y: self.y * r }
    }

    /// Returns perpendicular vector.
    pub fn perp(&self) -> Self { Self { x: -self.y, y: self.x } }

    /// Returns dot product of two vectors.
    pub fn dot(&self, rhs: Vf2d) -> f32 { self.x * rhs.x + self.y * rhs.y }

    /// Returns cross product of two vectors.
    pub fn cross(&self, rhs: Vf2d) -> f32 { self.x * rhs.y - self.y * rhs.x }
}

impl<T> From<(T, T)> for V2d<T> {
    fn from(tuple: (T, T)) -> Self {
        Self { x: tuple.0, y: tuple.1 }
    }
}
impl<T: ops::Add<Output=T>> ops::Add for V2d<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}
impl<T: ops::Add<Output=T> + Copy> ops::Add<T> for V2d<T> {
    type Output = Self;
    fn add(self, other: T) -> Self::Output {
        Self { x: self.x + other, y: self.y + other }
    }
}

impl<T: ops::AddAssign> ops::AddAssign for V2d<T> {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}
impl<T: ops::AddAssign + Copy> ops::AddAssign<T> for V2d<T> {
    fn add_assign(&mut self, other: T) {
        self.x += other;
        self.y += other;
    }
}


impl<T: ops::Sub<Output=T>> ops::Sub for V2d<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self { x: self.x - other.x, y: self.y - other.y }
    }
}
impl<T: ops::Sub<Output=T> + Copy> ops::Sub<T> for V2d<T> {
    type Output = Self;

    fn sub(self, other: T) -> Self::Output {
        Self { x: self.x - other, y: self.y - other }
    }
}

impl<T: ops::SubAssign> ops::SubAssign for V2d<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}
impl<T: ops::SubAssign + Copy> ops::SubAssign<T> for V2d<T> {
    fn sub_assign(&mut self, other: T) {
        self.x -= other;
        self.y -= other;
    }
}

impl<T: ops::Mul<Output=T>> ops::Mul for V2d<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self { x: self.x * other.x, y: self.y * other.y }
    }
}
impl<T: ops::Mul<Output=T> + Copy> ops::Mul<T> for V2d<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self::Output {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl<T: ops::MulAssign> ops::MulAssign for V2d<T> {
    fn mul_assign(&mut self, other: Self) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl<T: ops::MulAssign + Copy> ops::MulAssign<T> for V2d<T> {
    fn mul_assign(&mut self, other: T) {
        self.x *= other;
        self.y *= other;
    }
}

impl<T: ops::Div<Output=T>> ops::Div for V2d<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self { x: self.x / other.x, y: self.y / other.y }
    }
}
impl<T: ops::Div<Output=T> + Copy> ops::Div<T> for V2d<T> {
    type Output = Self;

    fn div(self, other: T) -> Self::Output {
        Self { x: self.x / other, y: self.y / other }
    }
}

impl<T: ops::DivAssign> ops::DivAssign for V2d<T> {
    fn div_assign(&mut self, other: Self) {
        self.x /= other.x;
        self.y /= other.y;
    }
}
impl<T: ops::DivAssign + Copy> ops::DivAssign<T> for V2d<T> {
    fn div_assign(&mut self, other: T) {
        self.x /= other;
        self.y /= other;
    }
}

impl<T: fmt::Display + fmt::Debug> fmt::Display for V2d<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HWButton {
    /// Set once during the frame the event occurs.
    pub pressed: bool,
    /// Set once during the frame the event occurs.
    pub released: bool,
    /// Set true for all frames between pressed and released events.
    pub held: bool
}

impl HWButton {
    fn new() -> Self {
        HWButton {
            pressed: false,
            released: false,
            held: false,
        }
    }
}

macro_rules! map (
    { $($key:expr => $value:expr),+ } => {
        {
            let m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keys {}

impl Keys {
    pub const NONE: i32 = 0x00;
    pub const A: i32 = 0x41;
    pub const B: i32 = 0x42;
    pub const C: i32 = 0x43;
    pub const D: i32 = 0x44;
    pub const E: i32 = 0x45;
    pub const F: i32 = 0x46;
    pub const G: i32 = 0x47;
    pub const H: i32 = 0x48;
    pub const I: i32 = 0x49;
    pub const J: i32 = 0x4A;
    pub const K: i32 = 0x4B;
    pub const L: i32 = 0x4C;
    pub const M: i32 = 0x4D;
    pub const N: i32 = 0x4E;
    pub const O: i32 = 0x4F;
    pub const P: i32 = 0x50;
    pub const Q: i32 = 0x51;
    pub const R: i32 = 0x52;
    pub const S: i32 = 0x53;
    pub const T: i32 = 0x54;
    pub const U: i32 = 0x55;
    pub const V: i32 = 0x56;
    pub const W: i32 = 0x57;
    pub const X: i32 = 0x58;
    pub const Y: i32 = 0x59;
    pub const Z: i32 = 0x5A;
    pub const F1: i32 = VK_F1;
    pub const F2: i32 = VK_F2;
    pub const F3: i32 = VK_F3;
    pub const F4: i32 = VK_F4;
    pub const F5: i32 = VK_F5;
    pub const F6: i32 = VK_F6;
    pub const F7: i32 = VK_F7;
    pub const F8: i32 = VK_F8;
    pub const F9: i32 = VK_F9;
    pub const F10: i32 = VK_F10;
    pub const F11: i32 = VK_F11;
    pub const F12: i32 = VK_F12;
    pub const DOWN: i32 = VK_DOWN;
    pub const LEFT: i32 = VK_LEFT;
    pub const RIGHT: i32 = VK_RIGHT;
    pub const UP: i32 = VK_UP;
    pub const ENTER: i32 = VK_RETURN;
    pub const BACK: i32 = VK_BACK;
    pub const ESCAPE: i32 = VK_ESCAPE;
    pub const PAUSE: i32 = VK_PAUSE;
    pub const SCROLL: i32 = VK_SCROLL;
    pub const TAB: i32 = VK_TAB;
    pub const DEL: i32 = VK_DELETE;
    pub const HOME: i32 = VK_HOME;
    pub const END: i32 = VK_END;
    pub const PGUP: i32 = VK_PRIOR;
    pub const PGDN: i32 = VK_NEXT;
    pub const INS: i32 = VK_INSERT;
    pub const SHIFT: i32 = VK_SHIFT;
    pub const CTRL: i32 = VK_CONTROL;
    pub const SPACE: i32 = VK_SPACE;
    pub const K0: i32 = 0x30;
    pub const K1: i32 = 0x31;
    pub const K2: i32 = 0x32;
    pub const K3: i32 = 0x33;
    pub const K4: i32 = 0x34;
    pub const K5: i32 = 0x35;
    pub const K6: i32 = 0x36;
    pub const K7: i32 = 0x37;
    pub const K8: i32 = 0x38;
    pub const K9: i32 = 0x39;
    pub const NP0: i32 = VK_NUMPAD0;
    pub const NP1: i32 = VK_NUMPAD1;
    pub const NP2: i32 = VK_NUMPAD2;
    pub const NP3: i32 = VK_NUMPAD3;
    pub const NP4: i32 = VK_NUMPAD4;
    pub const NP5: i32 = VK_NUMPAD5;
    pub const NP6: i32 = VK_NUMPAD6;
    pub const NP7: i32 = VK_NUMPAD7;
    pub const NP8: i32 = VK_NUMPAD8;
    pub const NP9: i32 = VK_NUMPAD9;
    pub const NP_MUL: i32 = VK_MULTIPLY;
    pub const NP_ADD: i32 = VK_ADD;
    pub const NP_DIV: i32 = VK_DIVIDE;
    pub const NP_SUB: i32 = VK_SUBTRACT;
    pub const NP_DECIMAL: i32 = VK_DECIMAL;
    pub const PERIOD: i32 = VK_DECIMAL;

    pub fn map_keys(k: Key) -> i32 {
        match k {
            Key::NONE => Keys::NONE,
            Key::A => Keys::A,
            Key::B => Keys::B,
            Key::C => Keys::C,
            Key::D => Keys::D,
            Key::E => Keys::E,
            Key::F => Keys::F,
            Key::G => Keys::G,
            Key::H => Keys::H,
            Key::I => Keys::I,
            Key::J => Keys::J,
            Key::K => Keys::K,
            Key::L => Keys::L,
            Key::M => Keys::M,
            Key::N => Keys::N,
            Key::O => Keys::O,
            Key::P => Keys::P,
            Key::Q => Keys::Q,
            Key::R => Keys::R,
            Key::S => Keys::S,
            Key::T => Keys::T,
            Key::U => Keys::U,
            Key::V => Keys::V,
            Key::W => Keys::W,
            Key::X => Keys::X,
            Key::Y => Keys::Y,
            Key::Z => Keys::Z,
            Key::F1 => Keys::F1,
            Key::F2 => Keys::F2,
            Key::F3 => Keys::F3,
            Key::F4 => Keys::F4,
            Key::F5 => Keys::F5,
            Key::F6 => Keys::F6,
            Key::F7 => Keys::F7,
            Key::F8 => Keys::F8,
            Key::F9 => Keys::F9,
            Key::F10 => Keys::F10,
            Key::F11 => Keys::F11,
            Key::F12 => Keys::F12,
            Key::DOWN => Keys::DOWN,
            Key::LEFT => Keys::LEFT,
            Key::RIGHT => Keys::RIGHT,
            Key::UP => Keys::UP,
            Key::ENTER => Keys::ENTER,
            Key::BACK => Keys::BACK,
            Key::ESCAPE => Keys::ESCAPE,
            Key::PAUSE => Keys::PAUSE,
            Key::SCROLL => Keys::SCROLL,
            Key::TAB => Keys::TAB,
            Key::DEL => Keys::DEL,
            Key::HOME => Keys::HOME,
            Key::END => Keys::END,
            Key::PGUP => Keys::PGUP,
            Key::PGDN => Keys::PGDN,
            Key::INS => Keys::INS,
            Key::SHIFT => Keys::SHIFT,
            Key::CTRL => Keys::CTRL,
            Key::SPACE => Keys::SPACE,
            Key::K0 => Keys::K0,
            Key::K1 => Keys::K1,
            Key::K2 => Keys::K2,
            Key::K3 => Keys::K3,
            Key::K4 => Keys::K4,
            Key::K5 => Keys::K5,
            Key::K6 => Keys::K6,
            Key::K7 => Keys::K7,
            Key::K8 => Keys::K8,
            Key::K9 => Keys::K9,
            Key::NP0 => Keys::NP0,
            Key::NP1 => Keys::NP1,
            Key::NP2 => Keys::NP2,
            Key::NP3 => Keys::NP3,
            Key::NP4 => Keys::NP4,
            Key::NP5 => Keys::NP5,
            Key::NP6 => Keys::NP6,
            Key::NP7 => Keys::NP7,
            Key::NP8 => Keys::NP8,
            Key::NP9 => Keys::NP9,
            Key::NP_MUL => Keys::NP_MUL,
            Key::NP_ADD => Keys::NP_ADD,
            Key::NP_DIV => Keys::NP_DIV,
            Key::NP_SUB => Keys::NP_SUB,
            Key::NP_DECIMAL => Keys::NP_DECIMAL,
            Key::PERIOD => Keys::PERIOD,
            _ => 0x00,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    NONE,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    K0,
    K1,
    K2,
    K3,
    K4,
    K5,
    K6,
    K7,
    K8,
    K9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    UP,
    DOWN,
    LEFT,
    RIGHT,
    SPACE,
    TAB,
    SHIFT,
    CTRL,
    INS,
    DEL,
    HOME,
    END,
    PGUP,
    PGDN,
    BACK,
    ESCAPE,
    RETURN,
    ENTER,
    PAUSE,
    SCROLL,
    NP0,
    NP1,
    NP2,
    NP3,
    NP4,
    NP5,
    NP6,
    NP7,
    NP8,
    NP9,
    NP_MUL,
    NP_DIV,
    NP_ADD,
    NP_SUB,
    NP_DECIMAL,
    PERIOD
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2d<T> {
    pub x: T,
    pub y: T
}

pub type Vi2d = V2d<i32>;
pub type Vf2d = V2d<f32>;

#[derive(Clone, Copy)]
pub union Pixel {
    n: u32,
    rgba: (u8, u8, u8, u8, )
}

impl Pixel {
    pub const GREY: Pixel = Pixel::rgb(192, 192, 192);
    pub const DARK_GREY: Pixel = Pixel::rgb(128, 128, 128);
    pub const VERY_DARK_GREY: Pixel = Pixel::rgb(64, 64, 64);
    pub const RED: Pixel = Pixel::rgb(255, 0, 0);
    pub const DARK_RED: Pixel = Pixel::rgb(128, 0, 0);
    pub const VERY_DARK_RED: Pixel = Pixel::rgb(64, 0, 0);
    pub const YELLOW: Pixel = Pixel::rgb(255, 255, 0);
    pub const DARK_YELLOW: Pixel = Pixel::rgb(128, 128, 0);
    pub const VERY_DARK_YELLOW: Pixel = Pixel::rgb(64, 64, 0);
    pub const GREEN: Pixel = Pixel::rgb(0, 255, 0);
    pub const DARK_GREEN: Pixel = Pixel::rgb(0, 128, 0);
    pub const VERY_DARK_GREEN: Pixel = Pixel::rgb(0, 64, 0);
    pub const CYAN: Pixel = Pixel::rgb(0, 255, 255);
    pub const DARK_CYAN: Pixel = Pixel::rgb(0, 128, 128);
    pub const VERY_DARK_CYAN: Pixel = Pixel::rgb(0, 64, 64);
    pub const BLUE: Pixel = Pixel::rgb(0, 0, 255);
    pub const DARK_BLUE: Pixel = Pixel::rgb(0, 0, 128);
    pub const VERY_DARK_BLUE: Pixel = Pixel::rgb(0, 0, 64);
    pub const MAGENTA: Pixel = Pixel::rgb(255, 0, 255);
    pub const DARK_MAGENTA: Pixel = Pixel::rgb(128, 0, 128);
    pub const VERY_DARK_MAGENTA: Pixel = Pixel::rgb(64, 0, 64);
    pub const WHITE: Pixel = Pixel::rgb(255, 255, 255);
    pub const BLACK: Pixel = Pixel::rgb(0, 0, 0);
    pub const BLANK: Pixel = Pixel::rgba(0, 0, 0, 0);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PixelMode {
    Normal,
    Mask,
    Alpha,
    Custom,
}

#[derive(Clone)]
pub struct Sprite {
    mode_sample: SpriteMode,
    pub width: u32,
    pub height: u32,
    col_data: Vec<Pixel>,
}

impl Sprite {
    pub fn new(width: u32, height: u32) -> Sprite {
        let mut image_size = (width * height) as usize;
        Sprite {
            mode_sample: SpriteMode::Normal,
            width,
            height,
            col_data: vec![Pixel::rgb(0, 0, 0); image_size]
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Pixel {
        match self.mode_sample {
            SpriteMode::Normal => {
                if x < self.width && y < self.height {
                    let index: usize = (y * self.width + x) as usize;
                    self.col_data[index]
                } else {
                    Pixel::rgb(0, 0, 0)
                }
            }
            SpriteMode::Periodic => {
                let index: usize = ((y % self.height) * self.width + (x % self.width)) as usize;
                self.col_data[index]
            }
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, p: Pixel) -> bool {
        if x < self.width && y < self.height {
            self.col_data[(y * self.width + x) as usize] = p;
            true
        } else {
            false
        }
    }

    pub fn sample(&self, x: f32, y: f32) -> Pixel {
        let sx: u32 = std::cmp::min((x * self.width as f32) as u32, self.width - 1);
        let sy: u32 = std::cmp::min((y * self.height as f32) as u32, self.height - 1);
        self.get_pixel(sx, sy)
    }

    pub fn sample_bl(u: f32, v: f32) -> Pixel {
        Pixel::rgb(0, 0, 0)
    }

    pub fn get_data(&mut self) -> &Vec<Pixel> {
        &self.col_data
    }

    pub fn overwrite_from_file<T: ImageLoader>(&mut self, file_path: &str)
                                               -> Rcode {
        T::load_image_resource(self, file_path)
    }

    pub fn load_from_file<T: ImageLoader>(file_path: &str) -> Self {
        let mut spr = Sprite::new(0, 0);
        T::load_image_resource(&mut spr, file_path);
        spr
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteMode {
    Normal,
    Periodic
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteFlip {
    None,
    Horiz,
    Vert
}

#[derive(Clone)]
pub struct SmallD {
    pub id: i32,
    pub sprite: Sprite,
    uv_scale: Vf2d
}

#[derive(Clone)]
pub struct Decal {
    d_inst: Arc<SmallD>,
}

impl Drop for Decal {
    fn drop(&mut self) {
        let mut id = self.get().id as u32;
        Renderer::delete_texture(&mut id);
    }
}

impl Decal {
    pub fn new() -> Self {
        let small = SmallD {
            id: -1,
            sprite: Sprite::new(0, 0),
            uv_scale: Vf2d::from((1.0, 1.0)),
        };
        Self {
            d_inst: Arc::new(small),
        }
    }

    pub fn create(spr: Option<Sprite>) -> Self {
        if let None = spr {
            Decal::new()
        } else {
            let sprite = spr.unwrap();
            let mut small = SmallD {
                id: Renderer::create_texture(sprite.width,
                                             sprite.height) as i32,
                sprite: sprite,
                uv_scale: Vf2d::from((1.0, 1.0)),
            };
            Decal::update(&mut small);
            let mut decal = Self {
                d_inst: Arc::new(small),
            };
            decal
        }
    }

    fn update(small: &mut SmallD) {
        if small.id == -1 { return; };
        small.uv_scale = Vf2d::from(
            (1.0 / (small.sprite.width as f32),
             (1.0 / (small.sprite.height as f32))
            ));
        Renderer::apply_texture(small.id as u32);
        Renderer::update_texture(small.id as u32, &small.sprite);
    }

    pub fn get(&self) -> Arc<SmallD> {
        Arc::clone(&self.d_inst)
    }
}

#[derive(Clone)]
pub struct DecalInstance {
    pub decal: Option<Arc<SmallD>>,
    pub pos: [Vf2d; 4],
    pub uv: [Vf2d; 4],
    pub w: [f32; 4],
    pub tint: [Pixel; 4],
}

impl DecalInstance {
    pub fn new() -> Self {
        Self {
            decal: None,
            pos: [Vf2d::from((0.0, 0.0)); 4],
            uv: [Vf2d::from((0.0, 0.0)), Vf2d::from((0.0, 1.0)),
                Vf2d::from((1.0, 1.0)), Vf2d::from((1.0, 0.0))],
            w: [1.0; 4],
            tint: [Pixel::rgb(255, 255, 255); 4]
        }
    }

    fn get(&self) -> &Arc<SmallD> {
        self.decal.as_ref().unwrap()
    }
}

#[derive(Clone)]
pub struct DecalTriangleInstance {
    pub decal: Decal,
    pub points: [Vf2d; 3],
    pub texture: [Vf2d; 3],
    pub colours: [Pixel; 3],
}

impl DecalTriangleInstance {
    pub fn new() -> Self {
        Self {
            decal: Decal::new(),
            points: [Vf2d::from((0.0, 0.0)); 3],
            texture: [Vf2d::from((0.0, 0.0)); 3],
            colours: [Pixel::rgb(255, 255, 255); 3],
        }
    }
}

#[derive(Clone)]
pub struct LayerDesc {
    pub id: u32,
    pub offset: Vf2d,
    pub scale: Vf2d,
    pub tint: Pixel,
    pub shown: bool,
    pub sprite: Sprite,
    pub update: bool,
    pub vec_decal_instance: Vec<DecalInstance>,
    pub func_hook: Option<fn(&mut LayerDesc)>,
}

impl LayerDesc {
    pub fn empty() -> Self {
        LayerDesc {
            id: 0,
            offset: Default::default(),
            scale: Default::default(),
            tint: Pixel::rgb(0, 0, 0),
            shown: false,
            sprite: Sprite::new(0, 0),
            update: false,
            vec_decal_instance: vec![],
            func_hook: None
        }
    }

    pub fn new(tex_w: u32, tex_h: u32) -> Self {
        LayerDesc {
            id: Renderer::create_texture(tex_w,
                                         tex_h),
            offset: Vf2d { x: 0.0, y: 0.0 },
            scale: Vf2d { x: 1.0, y: 1.0 },
            tint: Pixel::rgba(255, 255, 255, 255),
            shown: false,
            update: false,
            sprite: Sprite::new(tex_w,
                                tex_h),
            vec_decal_instance: vec![],
            func_hook: None
        }
    }
}

impl Pixel {
    /// Creates a new pixel with RGBA value.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            n: (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
        }
    }

    pub fn rand(mut rng: &mut ThreadRng) -> Self {
        let r: u32 = rng.gen_range(0, 255);
        let g: u32 = rng.gen_range(0, 255);
        let b: u32 = rng.gen_range(0, 255);
        let a: u32 = DEFAULT_ALPHA as u32;
        Self { n: (r | (g << 8) | (b << 16) | (a << 24)) as u32 }
    }

    /// Creates a new pixel with RGB value.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            n: (b as u32) | ((g as u32) << 8) | ((r as u32) << 16) | ((DEFAULT_ALPHA as
                u32) << 24)
        }
    }

    pub fn r(&self) -> u8 {
        unsafe { self.rgba.0 }
    }
    pub fn g(&self) -> u8 {
        unsafe { self.rgba.1 }
    }
    pub fn b(&self) -> u8 {
        unsafe { self.rgba.2 }
    }
    pub fn a(&self) -> u8 {
        unsafe { self.rgba.3 }
    }
}

pub trait ImageLoader {
    fn load_image_resource(spr: &mut Sprite, image_file: &str, ) -> Rcode;
    fn save_image_resource(spr: Sprite, image_file: &str) -> Rcode;
}

pub struct BMPLoader;

pub struct ResourceBuffer {}

pub struct ResourcePack {}

impl ImageLoader for BMPLoader {
    fn load_image_resource(spr: &mut Sprite, image_file: &str) -> Rcode {
        let image_path = std::path::Path::new(image_file);
        if !image_path.exists() { return Rcode::NoFile; }
        spr.col_data.clear();
        let img = bmp::open(image_path).unwrap_or_else(|e| {
            bmp::Image::new(0, 0)
        });
        if img.get_width() == 0 || img.get_height() == 0 { return Rcode::Fail; }
        spr.width = img.get_width();
        spr.height = img.get_height();
        //No Alpha for now because BMP is a dumb format
        spr.col_data = vec![Pixel::rgb(0, 0, 0); (spr.width * spr.height) as usize];
        for y in 0..spr.height {
            for x in 0..spr.width {
                let p = img.get_pixel(x, y);
                spr.set_pixel(x, y, Pixel::rgb(p.r, p.g, p.b));
            }
        }
        Rcode::Ok
    }
    fn save_image_resource(spr: Sprite, image_file: &str) -> Rcode {
        Rcode::Ok
    }
}

pub fn check_gl_error(i: i32) {
    let mut a = (GL.glGetError)();
    let mut errs = vec![];
    while a != 0 {
        errs.push(a);
        a = (GL.glGetError)();
    }
    if errs.len() != 0 {
        println!("Errors: {:?}, Position: {} ",
                 errs, i)
    }
}

pub type GLCallback = fn(source: u32, m_type: u32, id: u32, severity: u32,
                         length: u32, message: *const c_char,
                         userParam: *const usize);

pub fn gl_message_callback(source: u32, m_type: u32, id: u32, severity: u32,
                           length: u32, message: *const c_char,
                           userParam: *const usize) {
    unsafe {
        println!("GL CALLBACK: {} type = {:#X}, severity = {:#X}, message = {}",
                 (if m_type == 0x824C { "** GL ERROR **" } else { "" }),
                 m_type, severity, CStr::from_ptr(message).to_str().unwrap_or_default())
    }
}


macro_rules! gl_function {
    ($func_name:ident $(,$x:ty)* $(| $y:ty)*) => {
        unsafe{
            let glp = GLLoader::get_function_pointer(stringify!($func_name));
            let mut func: extern "C" fn ($($x),*) $(-> $y)* =
            std::mem::transmute(glp);
            func
        }
    };
}
macro_rules! gl_define {
    ($($x:ty,)* $(| $y:ty)*) => {
            extern "C" fn ($($x),*) $(-> $y)*
    };
}

pub struct GLLoader {
    wglSwapIntervalEXT: gl_define!( i32,),
    glEnable: gl_define!( u32,),
    glHint: gl_define!( u32, u32,),
    glViewport: gl_define!( u32, u32, u32, u32,),
    glClearColor: gl_define!( f32, f32, f32, f32,),
    glClear: gl_define!( u32,),
    glBlendFunc: gl_define!( u32, u32,),
    glGenTextures: gl_define!( u32, &mut u32,),
    glBindTexture: gl_define!( u32, u32,),
    glTexParameteri: gl_define!( u32, u32, u32,),
    glTexEnvi: gl_define!( u32, u32, u32,),
    glTexImage2D: gl_define!( u32, u32, u32, u32, u32, u32, u32, u32, *const usize,),
    glBegin: gl_define!( u32,),
    glColor4ub: gl_define!(u8, u8, u8, u8,),
    glTexCoord2f: gl_define!( f32, f32,),
    glVertex2f: gl_define!( f32, f32,),
    glDebugMessageCallback: gl_define!( GLCallback, u32,),
    glGetError: gl_define!(| i32),
    glEnd: gl_define!(),
    glDeleteTextures: gl_define!(u32, &mut u32,),
    glTexCoord4f: gl_define!(f32, f32, f32, f32,),
}

impl GLLoader {
    pub fn construct() -> Self {
        GLLoader {
            wglSwapIntervalEXT: gl_function!(wglSwapIntervalEXT, i32),
            glEnable: gl_function!(glEnable, u32),
            glHint: gl_function!(glHint, u32, u32),
            glViewport: gl_function!(glViewport, u32, u32, u32, u32),
            glClearColor: gl_function!(glClearColor, f32, f32, f32, f32),
            glClear: gl_function!(glClear, u32),
            glBlendFunc: gl_function!(glBlendFunc, u32, u32),
            glGenTextures: gl_function!(glGenTextures, u32, &mut u32),
            glBindTexture: gl_function!(glBindTexture, u32, u32),
            glTexParameteri: gl_function!(glTexParameteri, u32, u32, u32),
            glTexEnvi: gl_function!(glTexEnvi, u32, u32, u32),
            glTexImage2D: gl_function!(glTexImage2D, u32, u32, u32, u32, u32, u32, u32, u32, *const usize),
            glBegin: gl_function!(glBegin, u32),
            glEnd: gl_function!(glEnd),
            glColor4ub: gl_function!(glColor4ub,u8, u8, u8, u8),
            glTexCoord2f: gl_function!(glTexCoord2f, f32, f32),
            glVertex2f: gl_function!(glVertex2f, f32, f32),
            glDebugMessageCallback: gl_function!(glDebugMessageCallback, GLCallback, u32),
            glGetError: gl_function!(glGetError | i32),
            glDeleteTextures: gl_function!(glDeleteTextures, u32, &mut u32),
            glTexCoord4f: gl_function!(glTexCoord4f, f32, f32, f32, f32)
        }
    }

    pub fn get_function_pointer(func_name: &str) -> *const u64 {
        unsafe {
            let mut glp = wglGetProcAddress(CString::new(func_name).expect("Failed to get OpenGL function").as_ptr());

            if glp as *const u64 as u64 == 0 {
                let module: HMODULE = LoadLibraryA(CString::new("opengl32.dll").expect("Failed load OpenGL DLL").as_ptr());
                glp = GetProcAddress(module, CString::new(func_name).expect("Failed to get OpenGL function").as_ptr());
            }
            if glp as *const u64 as u64 == 0 {
                println!("FAILED TO LOAD OPENGL FUNCTION: {}", func_name);
            }
            glp as *const u64
        }
    }
}

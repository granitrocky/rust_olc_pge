use super::{
    olc::Olc,
    olc::OlcData,
    camera::Camera,
    decal::{Decal},
    engine::{OLCEngine},
    layer::{LayerDesc, LayerType, LayerFunc, LayerInfo},
    platform::{PLATFORM_DATA, Platform, PlatformWindows},
    renderer::Renderer,
    util::{HWButton,  Vf2d, Vi2d, RoundTo},
};

use std::time::UNIX_EPOCH;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub fn construct<T: 'static + Olc<D>, D: 'static + OlcData>(
    olc: T,
    game_data: D,
    app_name: &'static str,
    screen_width: u32,
    screen_height: u32,
    pixel_width: u32,
    pixel_height: u32,
    full_screen: bool,
    vsync: bool,
) {
    //Set the olc object to be used in this crate
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Warn);

    unsafe {
        PLATFORM_DATA.init();
        PLATFORM_DATA.resolution = Some(Vi2d::from((
            (screen_width / pixel_width) as i32,
            (screen_height / pixel_height) as i32,
        )));
        if !full_screen {
            PLATFORM_DATA.window_size =
                Some(Vi2d::from((screen_width as i32, screen_height as i32)));
        }
        PLATFORM_DATA.full_screen = full_screen;
        PLATFORM_DATA.title = app_name.into();
        PLATFORM_DATA.pixel_size = Some(Vi2d::new(pixel_width as i32, pixel_height as i32));
    };

    #[cfg(not(target_arch = "wasm32"))]
    futures::executor::block_on(finish_setup(
        olc,
        game_data,
        app_name,
        screen_width,
        screen_height,
        pixel_width,
        pixel_height,
        full_screen,
        vsync,
    ));

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(finish_setup(
        olc,
        game_data,
        app_name,
        screen_width,
        screen_height,
        pixel_width,
        pixel_height,
        full_screen,
        vsync,
    ));
}

async fn finish_setup<T: 'static + Olc<D>, D: 'static + OlcData>(
    olc: T,
    game_data: D,
    app_name: &'static str,
    screen_width: u32,
    screen_height: u32,
    pixel_width: u32,
    pixel_height: u32,
    full_screen: bool,
    vsync: bool,
) {
    let (window, event_loop) = PlatformWindows::create_window_pane(
        Vi2d { x: 10, y: 10 },
        unsafe { PLATFORM_DATA.window_size.unwrap() },
        unsafe { PLATFORM_DATA.full_screen },
    );

    #[cfg(target_arch = "wasm32")]
    {
        log::trace!("building canvas");
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();
        body.append_child(&canvas)
            .expect("Append canvas to HTML body");
    }
    let renderer: Renderer = Renderer::new(&window).await;

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
        renderer,
        game_data: Box::new(game_data),
        vsync: false,
        layers: vec![],
        draw_target: 0,
        mouse_position: Vi2d::from((0, 0)),
        font_decal: Decal::empty(),
        depth_buffer: vec![],
        camera: Camera::default(),
        window,
    };
    engine.init(
        app_name,
        screen_width,
        screen_height,
        pixel_width,
        pixel_height,
        full_screen,
        vsync,
    );
    start_game(olc, engine, event_loop).await;
}

async fn start_game<T: 'static + Olc<D>, D: OlcData + 'static>(
    olc: T,
    mut engine: OLCEngine<D>,
    event_loop: EventLoop<()>,
) {
    unsafe {
        if PLATFORM_DATA.full_screen {
            let fwin = PLATFORM_DATA.window_size.unwrap_or_default().to_vf2d();
            let fres = PLATFORM_DATA.resolution.unwrap_or_default().to_vf2d();
            PLATFORM_DATA.pixel_size = Some(
                (
                    (fwin.x as f32 / fres.x as f32) as i32,
                    (fwin.y as f32 / fres.y as f32) as i32,
                )
                    .into(),
            );
        }
    }
    engine.construct_font_sheet();
    engine.renderer.setup_layer_pipeline();
    engine.renderer.setup_3D_pipeline();
    //Create Primary Layer "0"
    let base_layer_id = engine.add_layer(LayerType::Image);
    let base_layer = engine.get_layer(base_layer_id).unwrap();
    engine.set_draw_target(base_layer_id);

    let mut frame_timer: f64 = 0.0;
    let mut frame_count: i32 = 0;
    let mut last_fps: i32 = 0;
    let mut frame_processed = true;
    let mut elapsed_time: f64 = 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    let mut game_timer = UNIX_EPOCH.elapsed().unwrap().as_secs_f64();

    #[cfg(target_arch = "wasm32")]
    let mut game_timer = js_sys::Date::now() as f64;

    //game_engine.construct_font_sheet();
    let mut engine = olc.on_engine_start(engine).unwrap().await;
    event_loop.run(move |top_event, window_target, control_flow| {
        *control_flow = ControlFlow::Poll;

        match top_event {
            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                if window_id == engine.window.id() {
                    if event == &WindowEvent::CloseRequested {
                        *control_flow = ControlFlow::Exit;
                    } else {
                        PlatformWindows::handle_window_event(&engine.window, &top_event);
                    }
                }
            }
            Event::RedrawEventsCleared => {
                engine.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                engine.renderer.active_decals = vec![];
                for layer in engine.layers.iter_mut() {
                    if let LayerInfo::Image(image_info) = &mut layer.layer_info {
                        if layer.shown {
                            engine
                                .renderer
                                .active_decals
                                .insert(layer.id as usize, layer.id);
                        }
                        if image_info.update {
                            engine
                                .renderer
                                .update_texture(layer.id as u32, &image_info.sprite);
                            image_info.update = false;
                        }
                    }
                }

                let mut encoder = engine.renderer.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    },
                );
                {
                    let clear_frames_render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[
                        //     wgpu::RenderPassColorAttachment{
                        //     view: &engine.renderer.frame_texture.view,
                        //     resolve_target: None,
                        //     ops: wgpu::Operations{
                        //         load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        //         store: true,
                        //     }
                        // },wgpu::RenderPassColorAttachment{
                        //     view: &engine.renderer.frame_texture_backbuffer.view,
                        //     resolve_target: None,
                        //     ops: wgpu::Operations{
                        //         load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        //         store: true,
                        //     }
                        // },
                        ],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &engine.renderer.depth_texture.texture_bundle.as_ref().unwrap().view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: true,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                        });
                }

                engine
                    .renderer
                    .queue
                    .submit(std::iter::once(encoder.finish()));
                engine.renderer.camera = engine.camera;
                //engine.renderer.draw_points(&engine.camera, &mut encoder);
                //engine.renderer.draw_mask(&engine.renderer.camera, Mask::D3, &engine.renderer.frame_texture_backbuffer, true, &mut encoder);

                let mut encoder = engine.renderer.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    },
                );
                let window_size = engine.get_window_size();
                let size = wgpu::Extent3d {
                    width: window_size.x as u32,
                    height: window_size.y as u32,
                    depth_or_array_layers: 1,
                };

                for (layer, function) in engine
                    .layers
                    .iter()
                    .filter_map(|layer| {
                        if layer.shown {
                            if let LayerInfo::Render(render_info) = &layer.layer_info {
                                Some((
                                    layer,
                                    &render_info
                                        .pipeline_bundle
                                        .as_ref()
                                        .expect("No Pipeline Info")
                                        .func,
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<(&LayerDesc<D>, &LayerFunc<D>)>>()
                {
                    function.execute(layer, &engine.renderer, &mut engine.game_data, &mut encoder);
                }

                //This pass will draw to the screen
                engine.renderer.draw_layers(&mut encoder);
                // submit will accept anything that implements IntoIter
                engine
                    .renderer
                    .queue
                    .submit(std::iter::once(encoder.finish()));

                #[cfg(not(target_arch = "wasm32"))]
                {elapsed_time = UNIX_EPOCH.elapsed().unwrap().as_secs_f64() - game_timer;}


                #[cfg(target_arch = "wasm32")]
                {elapsed_time = (js_sys::Date::now() as f64 - game_timer) / 1000.0;}

                #[cfg(not(target_arch = "wasm32"))]
                {
                    game_timer = UNIX_EPOCH.elapsed().unwrap().as_secs_f64();
                }

                #[cfg(target_arch = "wasm32")]
                {
                    game_timer = js_sys::Date::now() as f64;
                }
                engine.renderer.clear_frame();
                frame_timer += elapsed_time;
                frame_count += 1;
                if frame_timer >= 1.0 {
                    last_fps = frame_count;
                    engine.fps = (frame_count as f64 / frame_timer).floor() as u32;
                    let sTitle = engine.app_name.to_string()
                        + " - Avg Frame Time: "
                        + &((frame_timer / frame_count as f64) * 1000.0)
                            .round_to(2)
                            .to_string()
                        + " ms"
                        + " -- FPS: "
                        + &engine.fps.to_string();
                    PlatformWindows::set_window_title(&engine.window, sTitle);
                    frame_count = 0;
                    frame_timer -= 1.0;
                }
                update_inputs(&mut engine);
                frame_processed = true;
            }
            _ => {
                //#[cfg(not(target_arch = "wasm32"))]
                PlatformWindows::handle_window_event(&engine.window, &top_event);
            }
        }

        //Only run the engine if the last frame was drawn
        if frame_processed{
            engine.renderer.new_frame();
            if let Err(message) = olc.on_engine_update(&mut engine, elapsed_time) {
                log::error!("{}", message);
                println!("{}", message);
                *control_flow = ControlFlow::Exit;
            }
            engine.window.request_redraw();
            frame_processed = false;
        }
        //TODO: Setup physics engine on fixed time step
        //physics();

        engine.layers[0].shown = true;
    });
}

fn update_inputs<D: OlcData>(engine: &mut OLCEngine<D>){
    unsafe{
        let hw_func = |keys: &mut Vec<HWButton>,
        keys_old: &mut Vec<bool>,
        keys_new: &mut Vec<bool>,
        size: usize| {
            for i in 0..size as usize {
                keys[i].pressed = false;
                keys[i].released = false;
                if keys_new[i] != keys_old[i] {
                    if keys_new[i] {
                        keys[i].pressed = true;
                        keys[i].released = false;
                        keys[i].held = true;
                    } else {
                        keys[i].pressed = false;
                        keys[i].released = true;
                        keys[i].held = false;
                    }
                }
                keys_old[i] = keys_new[i];
            }
        };
        engine.clear_keys();
        for (key, value_new) in PLATFORM_DATA.new_key_state_map.as_mut().unwrap() {
            let value_old = PLATFORM_DATA
                .old_key_state_map
                .as_mut()
                .unwrap()
                .entry(*key)
                .or_insert(false);
            let current_key = PLATFORM_DATA
                .key_map
                .as_mut()
                .unwrap()
                .entry(*key)
                .or_insert_with(HWButton::new);
            if value_new != value_old {
                if *value_new {
                    (*current_key).pressed = true;
                    (*current_key).released = false;
                    (*current_key).held = true;
                } else {
                    (*current_key).pressed = false;
                    (*current_key).released = true;
                    (*current_key).held = false;
                }
            }
            *value_old = *value_new
        }

        hw_func(
            PLATFORM_DATA.mouse_map.as_mut().unwrap(),
            PLATFORM_DATA.old_mouse_state_map.as_mut().unwrap(),
            PLATFORM_DATA.new_mouse_state_map.as_mut().unwrap(),
            3,
        );
    }
    unsafe {

        let window_size = unsafe { PLATFORM_DATA.window_size.as_ref().unwrap() };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(pos) = PLATFORM_DATA.mouse_position_cache {
            PLATFORM_DATA.mouse_position = Some(pos);
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(pos) = PLATFORM_DATA.mouse_position_cache {
            PLATFORM_DATA.mouse_position = Some((pos.x, window_size.y as f32 - pos.y).into());
        }
        PLATFORM_DATA.mouse_wheel_delta = PLATFORM_DATA.mouse_wheel_delta_cache;
        PLATFORM_DATA.mouse_wheel_delta_cache = 0;
    }
}

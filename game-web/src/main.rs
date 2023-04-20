use std::panic;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::spawn_local;
use web_sys::UiEvent;
use engine::asset_resource::web::WithWebAssetSourceExt;
use engine::process::{ProcessBuilder, ProcessInfo};
use engine::resource::ResourceList;
use engine::winit_surface::{RunWinitSurfaceExt, WGPURenderExt, WinitSurfaceResource, WithWinitSurfaceExt};
use game::{run_game, setup_game};
use winit::platform::web::WindowExtWebSys;

// #[wasm_bindgen(start)]
fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).unwrap();

    log::info!("Building process...");

    spawn_local(async move {
        let base_url = web_sys::window()
            .unwrap()
            .location()
            .href()
            .unwrap();
        let base_url = web_sys::Url::new_with_base("assets/", &base_url)
            .unwrap()
            .href();

        ProcessBuilder::new(ProcessInfo)
            .with_winit_surface()
            .setup(|resources| {
                log::info!("Adding canvas element...");

                let winit_surface: &WinitSurfaceResource = resources.get();

                let window = winit_surface.window();
                let canvas = window.canvas();
                canvas.set_id("krill");

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let body = document.body().unwrap();

                body.append_child(&canvas).unwrap();

                log::info!("Added canvas element.");

                resources
            })
            .with_wgpu_render().await
            .with_web_asset_source(base_url)
            .setup_async(setup_game).await
            .run(run_game);
    });
}

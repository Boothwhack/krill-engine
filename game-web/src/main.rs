use std::panic;
use wasm_bindgen_futures::spawn_local;
use engine::asset_resource::web::WebRequestAssetSourceExt;
use engine::process::ProcessBuilder;
use engine::surface::{RunExt, SurfaceResource};
use engine::utils::HList;
use engine::utils::hlist::Has;
use engine::wgpu_render::{WGPUCompatible, WGPURenderSetupExt};
use engine::winit_surface::{WinitSetupExt, WinitSurface};
use winit::platform::web::WindowExtWebSys;
use game::{run_game, setup_game};

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

        ProcessBuilder::new()
            .setup_winit()
            .setup(|resources: HList!(SurfaceResource<WinitSurface>)| {
                log::info!("Adding canvas element...");

                let winit_surface: &SurfaceResource<WinitSurface> = resources.get();

                let window = winit_surface.raw_window();
                let canvas = window.canvas();
                canvas.set_id("krill");

                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let body = document.body().unwrap();

                body.append_child(&canvas).unwrap();

                log::info!("Added canvas element.");

                resources
            })
            .setup_wgpu_render().await
            .setup_web_request_asset_source(base_url)
            .setup_async(setup_game).await
            .run(run_game);
    });
}

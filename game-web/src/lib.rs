use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::spawn_local;
use engine::process::{ProcessBuilder, ProcessInfo};
use engine::winit_surface::{RunWinitSurfaceExt, WGPURenderExt, WithWinitSurfaceExt};
use game::{run_game, setup_game};

#[wasm_bindgen(start)]
fn main() {
    spawn_local(async {
        ProcessBuilder::new(ProcessInfo)
            .with_winit_surface()
            .with_wgpu_render().await
            .setup_async(setup_game).await
            .run(run_game);
    });
}

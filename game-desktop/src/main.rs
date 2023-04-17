use engine::asset_resource::desktop::WithDirectoryAssetSourceExt;
use engine::process::{ProcessBuilder, ProcessInfo};
use engine::resource::ResourceList;
use engine::winit_surface::{RunWinitSurfaceExt, WGPURenderExt, WinitSurfaceResource, WithWinitSurfaceExt};
use game::{run_game, setup_game};

#[tokio::main]
async fn main() {
    env_logger::builder().target(env_logger::Target::Stdout).init();

    ProcessBuilder::new(ProcessInfo)
        .with_winit_surface()
        .with_wgpu_render().await
        .with_directory_asset_source("assets")
        .setup_async(setup_game).await
        .run(run_game)
}

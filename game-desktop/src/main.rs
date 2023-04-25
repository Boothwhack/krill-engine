use engine::asset_resource::desktop::DirectoryAssetSourceExt;
use engine::process::ProcessBuilder;
use engine::surface::RunExt;
use engine::winit_surface::WinitSetupExt;
use engine::wgpu_render::WGPURenderSetupExt;
use game::{run_game, setup_game};

#[tokio::main]
async fn main() {
    env_logger::builder().target(env_logger::Target::Stdout).init();

    ProcessBuilder::new()
        .setup_winit()
        .setup_wgpu_render().await
        .setup_directory_asset_source("assets")
        .setup_async(setup_game).await
        .run(run_game);
}

use assets::source::desktop_fs::DirectoryAssetSource;
use engine::asset_resource::AssetSourceResource;
use engine::asset_resource::desktop::DirectoryAssetSourceExt;
use engine::process::ProcessBuilder;
use engine::surface::{RunExt, SurfaceEventResult};
use engine::utils::HList;
use engine::utils::hlist::{Has, ToMut};
use engine::winit_surface::WinitSetupExt;
use engine::wgpu_render::{WGPURenderResource, WGPURenderSetupExt};
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

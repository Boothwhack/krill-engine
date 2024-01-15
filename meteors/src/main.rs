use engine::platform::{detect_platform, Platform, SetupPlatformDefaultsExt, AsyncPlatform};
use engine::process::ProcessBuilder;
use engine::surface::RunExt;

mod game;
mod graphics;
mod text;

fn main() {
    #[cfg(target_family = "wasm")]
    console_log::init_with_level(log::Level::Debug).unwrap();

    #[cfg(not(target_family = "wasm"))]
    env_logger::builder().target(env_logger::Target::Stdout).init();

    let mut platform = detect_platform();

    #[cfg(target_family = "wasm")]
    platform.set_canvas_handler(|canvas| {
        use engine::platform::web::Placement;

        canvas.set_id("krill");
        Placement::Default(canvas)
    });

    platform.spawn_local(|mut platform| async move {
        let mut process = ProcessBuilder::new()
            .setup_platform_defaults(&mut platform).await
            .setup_async(game::setup_game_resources).await
            .build();

        process.event_system().handlers_for().append(game::on_surface_event);

        process.run();
    });
}

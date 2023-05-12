use log::Level;
use engine::events::Listeners;
use engine::platform::{detect_platform, Platform, SetupPlatformDefaultsExt};
use engine::process::ProcessBuilder;
use engine::surface::RunExt;

mod game;

fn main() {
    #[cfg(target_family = "wasm")]
    console_log::init_with_level(Level::Debug).unwrap();

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

        process.event_listeners(Listeners::new().with_listener(game::on_surface_event));

        process.run();
    });
}

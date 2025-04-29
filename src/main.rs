use bevy::prelude::*;

mod game;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (game::SCREEN_WIDTH, game::SCREEN_HEIGHT).into(),
            title: "Bevy Mine Sweeper".into(),
            ..Default::default()
        }),
        ..Default::default()
    }));

    #[cfg(feature = "debug")]
    app.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin {
        enable_multipass_for_primary_context: true,
    })
    .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());

    app.add_plugins(game::MineSweeper);

    app.run();
}

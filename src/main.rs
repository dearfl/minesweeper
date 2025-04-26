use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "debug")]
    app.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin {
        enable_multipass_for_primary_context: true,
    })
    .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());

    app.run();
}

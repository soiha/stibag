use crate::stibag::StibagGamePlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy::app::App;
use bevy::DefaultPlugins;
use bevy::prelude::PluginGroup;
use bevy::prelude::ImagePlugin;

mod stibag;

fn main() {
    println!("Hello, world!");
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins((StibagGamePlugin {}))
        .run();
}

use bevy_steam_audio::prelude::*;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(SpatialAudioPlugin)
        .add_startup_system(setup_listener)
        .add_startup_system(setup_sources)
        .run();
}

fn setup_listener(mut commands: Commands) {
    let listener = commands
        .spawn()
        .insert(Name::new("listener"))
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .id();

    commands.insert_resource(Listener(listener));
}

fn setup_sources(mut commands: Commands) {}

use bevy_steam_audio::{prelude::*, source::Listener};

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpatialAudioPlugin)
        .add_systems(Startup, setup_listener)
        .add_systems(Startup, setup_sources)
        .run();
}

fn setup_listener(mut commands: Commands) {
    let listener = commands
        .spawn(SpatialBundle::default())
        .insert(Listener)
        .insert(Name::new("listener"))
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .id();
}

fn setup_sources(mut commands: Commands) {}

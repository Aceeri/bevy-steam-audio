use std::sync::{Arc, Mutex};

use bevy::audio::AddAudioSource;
use bevy::audio::AudioPlugin;

use bevy::prelude::*;
use bevy_steam_audio::source::SineAudio;
use bevy_steam_audio::source::SpatialAudioPlugin;

#[derive(Resource)]
struct AudioHandles {
    eduardo: Handle<SineAudio>,
    direction_arcmut: Arc<Mutex<Vec3>>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AudioPlugin {
            global_volume: GlobalVolume::new(1.0),
        }))
        .add_audio_source::<SineAudio>()
        .add_plugins(SpatialAudioPlugin)
        .add_systems(Startup, setup_listener)
        .add_systems(Startup, setup_sources)
        .add_systems(Update, change_freq)
        .insert_resource(AudioHandles {
            eduardo: Handle::default(),
            direction_arcmut: Arc::default(),
        })
        .run();
}

fn setup_listener(mut commands: Commands) {
    let listener = commands
        .spawn(SpatialBundle::default())
        //.insert(Listener)
        .insert(Name::new("listener"))
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .id();
}

fn setup_sources(
    mut assets: ResMut<Assets<SineAudio>>,
    mut handles: ResMut<AudioHandles>,
    mut commands: Commands,
) {
    let some_val: Arc<Mutex<Vec3>> = Arc::new(Mutex::new(Vec3::default()));
    let some_val_ = some_val.clone();

    let audio_handle = assets.add(SineAudio { decoder: None, direction: some_val_ });

    handles.eduardo = audio_handle.clone();
    handles.direction_arcmut = some_val.clone();

    commands.spawn(AudioSourceBundle {
        source: audio_handle,
        ..default()
    });
}

fn change_freq(
    keyboard_input: Res<Input<KeyCode>>,
    handles: Res<AudioHandles>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::A) {
        println!("Just pressed");
        // commands.spawn(AudioSourceBundle {
        //     source: handles.eduardo.clone_weak(),
        //     ..default()
        // });
        let binding = handles.direction_arcmut.clone();
        let mut num = binding.lock().unwrap();
        *num += 1.0;
    }
}

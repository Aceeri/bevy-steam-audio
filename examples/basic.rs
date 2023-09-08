/// This example creates a scene with a camera (the listener) and a sound source in the middle.
/// The sound is spatialized with the Steam Audio HRTF
/// Fly around with W,A,S,D,Shift,Space and the mouse
/// Press F to start the sound again
use std::sync::{Arc, Mutex};

use bevy::audio::AddAudioSource;
use bevy::audio::AudioPlugin;

use bevy::prelude::*;
use bevy_steam_audio::source::SpatialAudioPlugin;
use bevy_steam_audio::source::SteamAudio;

use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransformPlugin,
};

#[derive(Resource)]
struct AudioHandles {
    eduardo: Handle<SteamAudio>,
    direction_arcmut: Arc<Mutex<Vec3>>,
}

#[derive(Component)]
struct ListenerSteam;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AudioPlugin {
            global_volume: GlobalVolume::new(1.0),
        }))
        .add_audio_source::<SteamAudio>()
        .add_plugins(SpatialAudioPlugin)
        .add_plugins(LookTransformPlugin)
        .add_plugins(FpsCameraPlugin::default())
        .add_systems(Startup, setup_sources)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (update_sound_direction, play_new_sound))
        .insert_resource(AudioHandles {
            eduardo: Handle::default(),
            direction_arcmut: Arc::default(),
        })
        .run();
}

fn setup_sources(
    mut assets: ResMut<Assets<SteamAudio>>,
    mut handles: ResMut<AudioHandles>,
    mut commands: Commands,
) {
    let source_direction: Arc<Mutex<Vec3>> = Arc::new(Mutex::new(Vec3::default()));
    let source_direction_ = source_direction.clone();

    let audio_handle = assets.add(SteamAudio {
        path: "assets/eduardo.ogg".to_owned(),
        direction: source_direction_,
    });

    handles.eduardo = audio_handle.clone();
    handles.direction_arcmut = source_direction.clone();

    commands.spawn(AudioSourceBundle {
        source: audio_handle,
        ..default()
    });
}

fn play_new_sound(
    keyboard_input: Res<Input<KeyCode>>,
    handles: Res<AudioHandles>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::F) {
        commands.spawn(AudioSourceBundle {
            source: handles.eduardo.clone_weak(),
            ..default()
        });
    }
}

fn update_sound_direction(
    handles: Res<AudioHandles>,
    listener_query: Query<&GlobalTransform, With<ListenerSteam>>,
) {
    let source_transform = GlobalTransform::default();

    let listener_transform = listener_query.get_single().unwrap();
    let local_transform = source_transform.reparented_to(listener_transform);

    let binding = handles.direction_arcmut.clone();
    let mut num = binding.lock().unwrap();

    *num = local_transform.translation.normalize_or_zero();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(0.2)),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands
        .spawn(Camera3dBundle::default())
        .insert(ListenerSteam)
        .insert(FpsCameraBundle::new(
            FpsCameraController::default(),
            Vec3::new(-2.0, 5.0, 5.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}

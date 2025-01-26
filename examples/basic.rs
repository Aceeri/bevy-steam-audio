/// This example creates a scene with a camera (the listener) and a sound source in the middle.
/// The sound is spatialized with the Steam Audio HRTF
/// Fly around with W,A,S,D,Shift,Space and the mouse
/// Press F to start the sound again
use std::sync::{Arc, Mutex};

use bevy::audio::AddAudioSource;
use bevy::audio::AudioPlugin;

use bevy::audio::SpatialScale;
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
}

#[derive(Component)]
struct ListenerSteam;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AudioPlugin {
            global_volume: GlobalVolume::new(1.0),
            default_spatial_scale: SpatialScale::new(1.0),
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

    let source_position: Arc<Mutex<Vec3>> = Arc::new(Mutex::new(Vec3::default()));
    let source_position_ = source_position.clone();

    let listener_position: Arc<Mutex<Vec3>> = Arc::new(Mutex::new(Vec3::default()));
    let listener_position_ = listener_position.clone();

    let audio_handle = assets.add(SteamAudio {
        path: "assets/eduardo.ogg".to_owned(),
        direction: source_direction_,
        source_position: source_position_,
        listener_position: listener_position_,
    });

    handles.eduardo = audio_handle.clone();

    commands.spawn(AudioPlayer(audio_handle));
}

fn play_new_sound(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    handles: Res<AudioHandles>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::KeyF) {
        commands.spawn(AudioPlayer(handles.eduardo.clone_weak()));
    }
}

fn update_sound_direction(
    handles: Res<AudioHandles>,
    assets: Res<Assets<SteamAudio>>,
    listener_query: Query<&GlobalTransform, With<ListenerSteam>>,
) {
    let source_transform = GlobalTransform::default(); // Todo
    let listener_transform = listener_query.get_single().unwrap();
    let local_transform = source_transform.reparented_to(listener_transform);

    let handle = assets.get(&handles.eduardo).unwrap();

    let binding = handle.direction.clone();
    let mut direction = binding.lock().unwrap();
    *direction = local_transform.translation.normalize_or_zero();

    let binding = handle.source_position.clone();
    let mut source_position = binding.lock().unwrap();
    *source_position = source_transform.translation();

    let binding = handle.listener_position.clone();
    let mut listener_position = binding.lock().unwrap();
    *listener_position = listener_transform.translation();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(0.2)),
    ));
    // light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands
        .spawn(Camera3d::default())
        .insert(ListenerSteam)
        .insert(FpsCameraBundle::new(
            FpsCameraController::default(),
            Vec3::new(-2.0, 5.0, 5.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}

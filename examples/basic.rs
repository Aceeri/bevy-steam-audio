use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bevy::ecs::event::ManualEventReader;
use bevy_steam_audio::source::SpatialAudioSettings;
use bevy_steam_audio::{prelude::*, source::Listener};

use bevy::audio::AddAudioSource;
use bevy::audio::AudioPlugin;
use bevy::audio::Source;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::Duration;

use bevy::prelude::*;
use steam_audio::interleave;

// This struct usually contains the data for the audio being played.
// This is where data read from an audio file would be stored, for example.
// Implementing `TypeUuid` will automatically implement `Asset`.
// This allows the type to be registered as an asset.
#[derive(TypePath, TypeUuid)]
#[uuid = "c2090c23-78fd-44f1-8508-c89b1f3cec29"]
struct SineAudio {
    // Reference to data // not using atm
    decoder: Option<f32>,
    direction: Arc<Mutex<Vec3>>,
}
// This decoder is responsible for playing the audio,
// and so stores data about the audio being played.
struct SineDecoder {
    // Reader
    decoder: rodio::Decoder<std::fs::File>,
    sample_rate: u32,
    current_channel: bool,
    current_block_offset: u32,
    current_block1: Vec<f32>,
    current_block2: Vec<f32>,
    binaural_params: BinauralParams,
    binaural_effect: BinauralEffect,
    settings: SpatialAudioSettings,
    blocks_played: u32,
    direction: Arc<Mutex<Vec3>>,
}

impl SineDecoder {
    // new(mut data)
    fn new(direction: Arc<Mutex<Vec3>>) -> Self {
        // Create reader
        let file = std::fs::File::open("assets/eduardo.ogg").unwrap();
        let dec = rodio::Decoder::new(file).unwrap();

        let audio_settings = AudioSettings::default();
        let context_settings = ContextSettings::default();
        let hrtf_settings = HRTFSettings::default();
        let simulation_settings = SimulationSettings::from_audio_settings(&audio_settings);

        let context = Context::new(&context_settings).expect("could not build steam audio context");
        let hrtf = HRTF::new(&context, &audio_settings, &hrtf_settings)
            .expect("could not build steam audio hrtf");
        let simulator = Simulator::new(&context, &simulation_settings)
            .expect("could not build steam audio simulation");

        let mut binaural_params = BinauralParams::default();
        binaural_params.interpolation = HRTFInterpolation::Bilinear;

        let binaural_effect = BinauralEffect::new(
            &context,
            &audio_settings,
            &hrtf,
        )
        .unwrap();

        // standard sample rate for most recordings
        let sample_rate = 44_100;
        SineDecoder {
            decoder: dec,
            sample_rate,
            current_channel: true,
            current_block_offset: 0,
            current_block1: Vec::new(),
            current_block2: Vec::new(),
            binaural_params,
            binaural_effect,
            settings: SpatialAudioSettings {
                audio_settings,
                context_settings,
                hrtf_settings,
                simulation_settings,
                context,
                hrtf,
                simulator,
            },
            blocks_played: 0,
            direction,
        }
    }
}

// copied from cpal samples_formats.rs. TODO: Replace this with the proper dependency
fn to_f32(sample: i16) -> f32 {
    if sample < 0 {
        sample as f32 / -(i16::MIN as f32)
    } else {
        sample as f32 / i16::MAX as f32
    }
}

// The decoder must implement iterator so that it can implement `Decodable`.
impl Iterator for SineDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        use glam::Vec3;
        
        loop {
            // todo: len() can be determined at creation
            if self.current_block_offset < self.current_block1.len() as u32 {
                // Read from the current block
                // let raw_val = self.current_block1[self.current_block_offset as usize];
                // self.current_block_offset += 1;
                // return Some(raw_val);
                let raw_val: f32;

                if self.current_channel {
                    raw_val = self.current_block1[self.current_block_offset as usize];
                } else {
                    raw_val = self.current_block2[self.current_block_offset as usize];
                    self.current_block_offset += 1;
                }

                self.current_channel = !self.current_channel;
                return Some(raw_val);
            }

            // Load the next block
            self.current_block_offset = 0;

            let mut input_buffer = DeinterleavedFrame::new(
                self.settings.audio_settings.frame_size() as usize,
                1,
                self.settings.audio_settings.sampling_rate(),
            );

            //input_buffer.push_source(&mut self.decoder); // TODO: Do we need this?? It was done once in the original example

            // move the stuff below to the struct?
            let mut output_buffer = DeinterleavedFrame::new(
                self.settings.audio_settings.frame_size() as usize,
                2,
                self.settings.audio_settings.sampling_rate(),
            );

            // todo: len() can be determined at creation
            if input_buffer.push_source(&mut self.decoder) {
                let dist = self.direction.lock().unwrap();
                println!("DistANCE: {}", *dist);
                let time =
                    (self.blocks_played as f32 / self.current_block1.len() as f32) * std::f32::consts::TAU * 15.0;

                self.binaural_params.direction = Vec3::new(time.cos(), 0.0, time.sin());

                self.binaural_effect
                    .apply_to_buffer(&self.binaural_params, &mut input_buffer, &mut output_buffer)
                    .unwrap();

                self.current_block1 = output_buffer.current_frame[0].clone();
                self.current_block2 = output_buffer.current_frame[1].clone();
                self.blocks_played += 1;
            } else {
                return None;
            }
        }
    }
}
// `Source` is what allows the audio source to be played by bevy.
// This trait provides information on the audio.
impl Source for SineDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// Finally `Decodable` can be implemented for our `SineAudio`.
impl Decodable for SineAudio {
    type Decoder = SineDecoder;

    type DecoderItem = <SineDecoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        SineDecoder::new(self.direction.clone())
    }
}

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
        .insert(Listener)
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

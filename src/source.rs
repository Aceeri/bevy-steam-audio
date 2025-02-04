use bevy::{
    app::{App, Plugin},
    asset::Asset,
    audio::Decodable,
    math::{Dir3, Vec3},
    prelude::{Component, GlobalTransform, Mesh, Query, Res, Resource, With},
    reflect::TypePath,
};
use std::sync::{Arc, Mutex};

use bevy::audio::Source;
use bevy::utils::Duration;

use steam_audio::{
    hrtf::{AudioSettings, HRTFInterpolation, HRTFSettings, HRTF},
    prelude::{
        BinauralEffect, BinauralParams, Context, ContextSettings, DeinterleavedFrame, DirectEffect,
        DirectEffectFlags, DirectEffectParams, DistanceAttenuationModel, SimulationFlags,
        SimulationSettings, SimulationSharedInputs, Simulator,
    },
    simulation::source::{AirAbsorptionModel, Directivity},
    Orientation,
};

use bevy::render::{
    mesh::{Indices, VertexAttributeValues},
    render_resource::PrimitiveTopology,
};

// This struct usually contains the data for the audio being played.
// This is where data read from an audio file would be stored, for example.
// Implementing `TypePath` will automatically implement `Asset`.
// This allows the type to be registered as an asset.
#[derive(TypePath, Asset)]
pub struct SteamAudio {
    pub path: String,
    pub direction: Arc<Mutex<Vec3>>,
    pub source_position: Arc<Mutex<Vec3>>,
    pub listener_position: Arc<Mutex<Vec3>>,
}

// This decoder is responsible for playing the audio,
// and so stores data about the audio being played.
pub struct SteamDecoder {
    // Reader
    decoder: rodio::Decoder<std::fs::File>,
    sample_rate: u32,
    current_channel: bool,
    current_block_offset: u32,
    current_block1: Vec<f32>,
    current_block2: Vec<f32>,
    binaural_params: BinauralParams,
    binaural_effect: BinauralEffect,
    direct_params: DirectEffectParams,
    direct_effect: DirectEffect,
    settings: SpatialAudioSettings,
    blocks_played: u32,
    direction: Arc<Mutex<Vec3>>,
    source_position: Arc<Mutex<Vec3>>,
    listener_position: Arc<Mutex<Vec3>>,
}

impl SteamDecoder {
    fn new(
        direction: Arc<Mutex<Vec3>>,
        source_position: Arc<Mutex<Vec3>>,
        listener_position: Arc<Mutex<Vec3>>,
        path: String,
    ) -> Self {
        // Create reader
        let file = std::fs::File::open(path).unwrap();
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

        let binaural_effect = BinauralEffect::new(&context, &audio_settings, &hrtf).unwrap();

        let mut direct_params = DirectEffectParams::default();
        direct_params.flags = DirectEffectFlags::AIR_ABSORPTION
            | DirectEffectFlags::DISTANCE_ATTENUATION
            | DirectEffectFlags::DIRECTIVITY;
        let direct_effect = DirectEffect::new(&context, &audio_settings, 1).unwrap();

        // standard sample rate for most recordings
        let sample_rate = 44_100;
        SteamDecoder {
            decoder: dec,
            sample_rate,
            current_channel: true,
            current_block_offset: 0,
            current_block1: Vec::new(),
            current_block2: Vec::new(),
            binaural_params,
            binaural_effect,
            direct_params,
            direct_effect,
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
            source_position,
            listener_position,
        }
    }
}

// The decoder must implement iterator so that it can implement `Decodable`.
impl Iterator for SteamDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // todo: len() can be determined at creation
            if self.current_block_offset < self.current_block1.len() as u32 {
                // Read from the current block
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

            let mut intermediate_buffer = DeinterleavedFrame::new(
                self.settings.audio_settings.frame_size() as usize,
                1,
                self.settings.audio_settings.sampling_rate(),
            );

            // move the stuff below to the struct?
            let mut output_buffer = DeinterleavedFrame::new(
                self.settings.audio_settings.frame_size() as usize,
                2,
                self.settings.audio_settings.sampling_rate(),
            );

            // todo: len() can be determined at creation
            if input_buffer.push_source(&mut self.decoder) {
                let dir: Vec3 = *self.direction.lock().unwrap();
                let source_pos: Vec3 = *self.source_position.lock().unwrap();
                let listener_pos: Vec3 = *self.listener_position.lock().unwrap();

                let attenuation_model = DistanceAttenuationModel::default();
                let attenuation = attenuation_model.calculate(
                    &self.settings.context,
                    source_pos.into(),
                    listener_pos.into(),
                );

                let absorption_model = AirAbsorptionModel::default();
                let absorption = absorption_model.calculate(
                    &self.settings.context,
                    source_pos.into(),
                    listener_pos.into(),
                );

                let directivity_model = Directivity {
                    dipole_weight: 0.0,
                    dipole_power: 1.0,
                };
                let directivity = directivity_model.calculate(
                    &self.settings.context,
                    Orientation {
                        right: Vec3::X.into(),
                        up: Vec3::Y.into(),
                        ahead: Vec3::NEG_Z.into(),
                        origin: Vec3::ZERO.into(),
                    },
                    listener_pos.into(),
                );

                self.direct_params.distance_attenuation = attenuation;
                self.direct_params.air_absorption = absorption;
                self.direct_params.directivity = directivity;

                // todo: why is direct effect apply_to_buffer input not mut compared to binaural effect?
                self.direct_effect
                    .apply_to_buffer(&self.direct_params, input_buffer, &mut intermediate_buffer)
                    .unwrap();

                self.binaural_params.direction = dir.into();

                self.binaural_effect
                    .apply_to_buffer(
                        &self.binaural_params,
                        &mut intermediate_buffer,
                        &mut output_buffer,
                    )
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
impl Source for SteamDecoder {
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

impl Decodable for SteamAudio {
    type DecoderItem = <SteamDecoder as Iterator>::Item;

    type Decoder = SteamDecoder;

    fn decoder(&self) -> Self::Decoder {
        SteamDecoder::new(
            self.direction.clone(),
            self.source_position.clone(),
            self.listener_position.clone(),
            self.path.clone(),
        )
    }
}

// Todo implement default
#[derive(Resource)]
pub struct SpatialAudioSettings {
    pub audio_settings: AudioSettings,
    pub context_settings: ContextSettings,
    pub hrtf_settings: HRTFSettings,
    pub simulation_settings: SimulationSettings,
    pub context: Context,
    pub hrtf: HRTF,
    pub simulator: Simulator,
}

pub struct SpatialAudioPlugin;

impl Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        let audio_settings = AudioSettings::default();
        let context_settings = ContextSettings::default();
        let hrtf_settings = HRTFSettings::default();
        let simulation_settings = SimulationSettings::from_audio_settings(&audio_settings);

        let context = Context::new(&context_settings).expect("could not build steam audio context");
        let hrtf = HRTF::new(&context, &audio_settings, &hrtf_settings)
            .expect("could not build steam audio hrtf");
        let simulator = Simulator::new(&context, &simulation_settings)
            .expect("could not build steam audio simulation");

        app.insert_resource(SpatialAudioSettings {
            audio_settings,
            context_settings,
            hrtf_settings,
            simulation_settings,
            context,
            hrtf,
            simulator,
        });
    }
}

// pub fn context_update(mut commands: Commands, settings: Res<ContextSettings>) {
//     if settings.is_changed() {
//         match Context::new(&*settings) {
//             Ok(context) => {
//                 commands.insert_resource(context);
//             }
//             _ => {}
//         }
//     }
// }

// pub fn hrtf_update(
//     mut commands: Commands,
//     context: Res<Context>,
//     audio_settings: Res<AudioSettings>,
//     hrtf_settings: Res<HRTFSettings>,
// ) {
//     if context.is_changed() || audio_settings.is_changed() || hrtf_settings.is_changed() {
//         match HRTF::new(&context, &audio_settings, &hrtf_settings) {
//             Ok(hrtf) => {
//                 commands.insert_resource(hrtf);
//             }
//             _ => {}
//         };
//     }
// }

// pub fn simulation_update(
//     mut commands: Commands,
//     context: Res<Context>,
//     simulation_settings: Res<SimulationSettings>,
// ) {
//     if context.is_changed() || simulation_settings.is_changed() {
//         match Simulator::new(&*context, &simulation_settings) {
//             Ok(simulator) => {
//                 commands.insert_resource(simulator);
//             }
//             _ => {}
//         }
//     }
// }

#[derive(Component)]
pub struct Listener;

pub fn listener_update(
    audio_resource: Res<SpatialAudioSettings>,
    query: Query<&GlobalTransform, With<Listener>>,
) {
    for transform in query.iter() {
        let flags = SimulationFlags::all();
        let orientation = Orientation {
            origin: transform.translation().into(),
            right: transform.right().as_array(),
            up: transform.up().as_array(),
            ahead: transform.forward().to_array(),
        };

        let shared_inputs = SimulationSharedInputs {
            listener: orientation,
            ..Default::default()
        };

        audio_resource
            .simulator
            .set_shared_inputs(flags, &shared_inputs);
    }
}

pub struct AudioMesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    pub materials: Vec<steam_audio::prelude::Material>,
    pub material_indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub enum AudioMeshError {
    NoVertices,
    NonTrianglePrimitiveTopology(PrimitiveTopology),
}

impl TryFrom<Mesh> for AudioMesh {
    type Error = AudioMeshError;
    fn try_from(mesh: Mesh) -> Result<Self, Self::Error> {
        let triangles = match mesh.indices() {
            Some(indices) => {
                let indices: Vec<_> = match indices {
                    Indices::U16(indices) => {
                        indices.iter().map(|indices| *indices as u32).collect()
                    }
                    Indices::U32(indices) => indices.iter().map(|indices| *indices).collect(),
                };

                match mesh.primitive_topology() {
                    PrimitiveTopology::TriangleList => indices
                        .chunks_exact(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                        .collect(),
                    PrimitiveTopology::TriangleStrip => {
                        let mut indices: Vec<_> = indices
                            .windows(3)
                            .map(|indices| [indices[0], indices[1], indices[2]])
                            .collect();

                        for (index, indices) in indices.iter_mut().enumerate() {
                            if (index + 1) % 2 == 0 {
                                *indices = [indices[1], indices[0], indices[2]];
                            }
                        }

                        indices
                    }
                    topology => return Err(AudioMeshError::NonTrianglePrimitiveTopology(topology)),
                }
            }
            None => Vec::new(),
        };

        let vertices = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(positions) => match positions {
                VertexAttributeValues::Float32x3(vertices) => {
                    vertices.iter().map(|a| (*a).into()).collect()
                }
                _ => return Err(AudioMeshError::NoVertices),
            },
            _ => return Err(AudioMeshError::NoVertices),
        };

        let materials = vec![steam_audio::materials::GENERIC];
        let material_indices = triangles.iter().map(|_| 0 /* GENERIC index */).collect();

        Ok(Self {
            vertices: vertices,
            triangles: triangles,
            materials: materials,
            material_indices: material_indices,
        })
    }
}

trait AsArray<const N: usize> {
    fn as_array(self) -> [f32; N];
}

impl AsArray<3> for Dir3 {
    fn as_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

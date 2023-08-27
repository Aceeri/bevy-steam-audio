use steam_audio::{prelude::*, Orientation};

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
};

use glam::Vec3;

// #[derive(Component, Debug)]
// pub struct SpatialAudioSource {
//     pub settings: SourceSettings,
// }

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

// Todo: Need to get rid of this...
fn glam_vec(input: bevy::prelude::Vec3) -> glam::Vec3 {
    glam::Vec3::new(input.x, input.y, input.z)
}

#[derive(Component)]
pub struct Listener;

pub fn listener_update(
    audio_resource: Res<SpatialAudioSettings>,
    query: Query<(&GlobalTransform, &Listener)>,
) {
    for (transform, listener_component) in query.iter() {
        let flags = SimulationFlags::all();
        let orientation = Orientation {
            origin: glam_vec(transform.translation()),
            right: glam_vec(transform.right()),
            up: glam_vec(transform.up()),
            ahead: glam_vec(transform.forward()),
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

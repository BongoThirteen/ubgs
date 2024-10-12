
mod noise_helpers;

use std::{collections::{hash_map::Entry, HashMap}, sync::Arc, thread::{available_parallelism, spawn}};

use flume::{unbounded, Receiver, Sender};
use noise::Perlin;
use noise_helpers::range_noise;
use valence::prelude::*;

const HEIGHT: u32 = 384;

pub struct WorldGen;

impl Plugin for WorldGen {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (send_recv_chunks, update_client_views));
    }
}

struct ChunkWorkerState {
    sender: Sender<(Entity, ChunkPos, UnloadedChunk)>,
    receiver: Receiver<(Entity, ChunkPos)>,
    noise: Perlin,
}

type Priority = u64;

#[derive(Resource)]
struct WorldGenState {
    pending: HashMap<(Entity, ChunkPos), Option<Priority>>,
    sender: Sender<(Entity, ChunkPos)>,
    receiver: Receiver<(Entity, ChunkPos, UnloadedChunk)>,
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let (finished_sender, finished_receiver) = unbounded();
    let (pending_sender, pending_receiver) = unbounded();

    let state = Arc::new(ChunkWorkerState {
        sender: finished_sender,
        receiver: pending_receiver,
        noise: Perlin::new(0),
    });

    for _ in 0..available_parallelism().unwrap().get() {
        let state = state.clone();
        spawn(move || chunk_worker(state));
    }

    commands.insert_resource(WorldGenState {
        pending: HashMap::new(),
        sender: pending_sender,
        receiver: finished_receiver,
    });

    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    commands.spawn(layer);
}

fn update_client_views(
    mut layers: Query<(Entity, &mut ChunkLayer)>,
    mut clients: Query<(&mut Client, View, OldView)>,
    mut state: ResMut<WorldGenState>,
) {
    let (layer_id, layer) = layers
        .iter_mut()
        .find(|(_, l)| l.dimension_type_name() == ident!("overworld"))
        .expect("no dimension by name `minecraft:overworld`");

    for (client, view, old_view) in &mut clients {
        let view = view.get();
        let queue_pos = |pos: ChunkPos| {
            if layer.chunk(pos).is_none() {
                match state.pending.entry((layer_id, pos)) {
                    Entry::Occupied(mut oe) => {
                        if let Some(priority) = oe.get_mut() {
                            let dist = view.pos.distance_squared(pos);
                            *priority = (*priority).min(dist);
                        }
                    }
                    Entry::Vacant(ve) => {
                        let dist = view.pos.distance_squared(pos);
                        ve.insert(Some(dist));
                    }
                }
            }
        };

        if client.is_added() {
            view.iter().for_each(queue_pos);
        } else {
            let old_view = old_view.get();
            if old_view != view {
                view.diff(old_view).for_each(queue_pos);
            }
        }
    }
}

fn send_recv_chunks(mut layers: Query<&mut ChunkLayer>, state: ResMut<WorldGenState>) {
    let state = state.into_inner();

    // Insert the chunks that are finished generating into the instance.
    for (layer_id, pos, chunk) in state.receiver.drain() {
        let Ok(mut layer) = layers.get_mut(layer_id) else {
            continue;
        };
        layer.insert_chunk(pos, chunk);
        assert!(state.pending.remove(&(layer_id, pos)).is_some());
    }

    // Collect all the new chunks that need to be loaded this tick.
    let mut to_send = vec![];

    for (pos, priority) in &mut state.pending {
        if let Some(pri) = priority.take() {
            to_send.push((pri, pos));
        }
    }

    // Sort chunks by ascending priority.
    to_send.sort_unstable_by_key(|(pri, _)| *pri);

    // Send the sorted chunks to be loaded.
    for (_, pos) in to_send {
        let _ = state.sender.try_send(*pos);
    }
}

fn chunk_worker(state: Arc<ChunkWorkerState>) {
    while let Ok((layer, pos)) = state.receiver.recv() {
        let mut chunk = UnloadedChunk::with_height(HEIGHT);

        // generate chunk
        terrain(&state.noise, pos, &mut chunk);
        
        let _ = state.sender.try_send((layer, pos, chunk));
    }
}

fn terrain(noise: &Perlin, pos: ChunkPos, chunk: &mut UnloadedChunk) {
    let _xz_scale = 0.9999999814507745;
    let _y_scale = 0.9999999814507745;
    let _xz_factor = 80.0;
    let _y_factor = 160.0;
    let _size_horizontal = 1;
    let _size_vertical = 2;

    let freq_x = 0.005221649073064327;
    let freq_y = 0.0026108245365321636;
    let freq_z = 0.005221649073064327;

    let base_height = 8.5 * 8.0;
    let height_bias = 0.005; // arbitrary

    for offset_z in 0..16 {
        for offset_x in 0..16 {
            let x = offset_x as i32 + pos.x * 16;
            let z = offset_z as i32 + pos.z * 16;

            for y in 0..384 {
                let pos = DVec3::new((x as f64 + 0.5) * freq_x, y as f64 * freq_y, (z as f64 + 0.5) * freq_z);

                let density = range_noise(noise, pos) - (y as f64 - base_height) * height_bias;

                if density > 0.0 {
                    chunk.set_block(offset_x, y, offset_z, BlockState::STONE);
                }
            }
        }
    }
}

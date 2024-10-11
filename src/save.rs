
use valence::prelude::*;
use crate::anvil::{AnvilLevel, AnvilPlugin, ChunkLoadEvent, ChunkLoadStatus};

pub struct Save;

impl Plugin for Save {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(AnvilPlugin)
            .add_systems(Startup, setup)
            .add_systems(Update, handle_chunk_loads);
    }
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new("world", &biomes);

    // Force a 16x16 area of chunks around the origin to be loaded at all times.
    // This is similar to "spawn chunks" in vanilla. This isn't necessary for the
    // example to function, but it's done to demonstrate that it's possible.
    for z in -8..8 {
        for x in -8..8 {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    commands.spawn((layer, level));
}

fn handle_chunk_loads(
    mut events: EventReader<ChunkLoadEvent>,
    mut layers: Query<&mut ChunkLayer, With<AnvilLevel>>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        match &event.status {
            ChunkLoadStatus::Success { .. } => {
                // The chunk was inserted into the world. Nothing for us to do.
            }
            ChunkLoadStatus::Empty => {
                // There's no chunk here so let's insert an empty chunk. If we were doing
                // terrain generation we would prepare that here.
                layer.insert_chunk(event.pos, UnloadedChunk::new());
            }
            ChunkLoadStatus::Failed(e) => {
                // Something went wrong.
                let errmsg = format!(
                    "failed to load chunk at ({}, {}): {e:#}",
                    event.pos.x, event.pos.z
                );

                eprintln!("{errmsg}");
                layer.send_chat_message(errmsg.color(Color::RED));

                layer.insert_chunk(event.pos, UnloadedChunk::new());
            }
        }
    }
}

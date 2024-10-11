
use valence::entity::tnt::{Fuse, TntEntityBundle};
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::nbt::Value;
use valence::prelude::*;

pub struct Explosion;

impl Plugin for Explosion {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ExplosionEvent>()
            .add_systems(Update, explode)
            .add_systems(Update, (ignite, fuse).before(explode));
    }
}

#[derive(Event)]
pub struct ExplosionEvent {
    position: DVec3,
}

fn ignite(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    mut commands: Commands,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((mut inventory, game_mode, held, entity_layer)) = clients.get_mut(event.client) else {
            continue;
        };

        let slot_id = held.slot();
        let mut stack = inventory.slot(slot_id).clone();

        if stack.item != ItemKind::FlintAndSteel || !layer.block(event.position).is_some_and(|b| b.state.to_kind() == BlockKind::Tnt) {
            continue;
        }

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if let Some(damage) = stack.nbt.as_ref().and_then(|nbt| match nbt.get("Damage") {
                Some(&Value::Int(d)) => Some(d),
                _ => None,
            }).filter(|d| *d < 63) {
                stack.nbt = stack.nbt.map(|mut nbt| { nbt.insert("Damage", Value::Int(damage + 1)); nbt });
                inventory.set_slot(slot_id, stack);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }

        layer.set_block(event.position, BlockState::AIR);

        let position = Position(DVec3::new(f64::from(event.position.x) + 0.5, event.position.y.into(), f64::from(event.position.z) + 0.5));

        commands.spawn(TntEntityBundle {
            position,
            layer: *entity_layer,
            ..Default::default()
        });
    }
}

fn fuse(
    mut fuses: Query<(Entity, &mut Fuse, &Position)>,
    mut events: EventWriter<ExplosionEvent>,
    mut commands: Commands,
) {
    for (tnt, mut fuse, &Position(position)) in &mut fuses {
        if fuse.0 > 0 {
            fuse.0 -= 1;
        } else if let Some(mut entity) = commands.get_entity(tnt) {
            entity.insert(Despawned);
            events.send(ExplosionEvent { position });
        }
    }
}

fn explode(
    mut events: EventReader<ExplosionEvent>,
    mut layers: Query<&mut ChunkLayer>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        for dx in -3..=3 {
            for dy in -3..=3 {
                for dz in -3..=3 {
                    let pos = BlockPos {
                        x: event.position.x as i32 + dx,
                        y: event.position.y as i32 + dy,
                        z: event.position.z as i32 + dz,
                    };
                    layer.set_block(pos, BlockState::AIR);
                }
            }
        }
    }
}

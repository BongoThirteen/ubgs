
use avian3d::collision::Collider;
use avian3d::position;
use avian3d::prelude::LinearVelocity;
use avian3d::spatial_query::{RayCaster, RayHits};
use bevy::math::Dir3;
use valence::entity::tnt::{Fuse, TntEntityBundle};
use valence::entity::Velocity;
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::nbt::Value;
use valence::prelude::*;
use valence::rand::{thread_rng, Rng};

use crate::block_update::BlockUpdateEvent;

pub struct Explosion;

impl Plugin for Explosion {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ExplosionEvent>()
            .add_systems(Update, (explode, knockback))
            .add_systems(Update, (ignite, fuse).before(explode));
    }
}

#[derive(Event)]
pub struct ExplosionEvent {
    position: DVec3,
    layer: EntityLayerId,
}

fn ignite(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    mut updates: EventWriter<BlockUpdateEvent>,
    mut commands: Commands,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((mut inventory, game_mode, held, &entity_layer)) = clients.get_mut(event.client) else {
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

        updates.send(BlockUpdateEvent { position: event.position, layer: entity_layer.0, entity_layer });
        for dir in [Direction::Up, Direction::Down, Direction::North, Direction::East, Direction::South, Direction::West] {
            updates.send(BlockUpdateEvent { position: event.position.get_in_direction(dir), layer: entity_layer.0, entity_layer });
        }

        let position = Position(DVec3::new(f64::from(event.position.x) + 0.5, event.position.y.into(), f64::from(event.position.z) + 0.5));

        let angle = thread_rng().gen_range(0.0..std::f32::consts::TAU);
        let dir = Vec3::new(angle.cos() * 0.01, 0.01, angle.sin() * 0.01);

        commands.spawn(TntEntityBundle {
            position,
            layer: entity_layer,
            velocity: Velocity(dir),
            ..Default::default()
        });
    }
}

fn fuse(
    mut fuses: Query<(Entity, &mut Fuse, &Position, &EntityLayerId)>,
    mut events: EventWriter<ExplosionEvent>,
    mut commands: Commands,
) {
    for (tnt, mut fuse, &Position(position), &layer) in &mut fuses {
        if fuse.0 > 0 {
            fuse.0 -= 1;
        } else if let Some(mut entity) = commands.get_entity(tnt) {
            entity.insert(Despawned);
            events.send(ExplosionEvent { position, layer });
        }
    }
}

#[derive(Component)]
struct ExplosionKnockback(f64);

fn explode(
    mut events: EventReader<ExplosionEvent>,
    mut layers: Query<&mut ChunkLayer>,
    colliders: Query<(Entity, &position::Position), (With<Collider>, Without<Hitbox>)>,
    mut commands: Commands,
) {
    // for event in events.read() {
    //     for dy in -3..=3 {
    //         for dz in -3..=3 {
    //             for dx in -3..=3 {
    //                 let pos = BlockPos {
    //                     x: event.position.x as i32 + dx,
    //                     y: event.position.y as i32 + dy,
    //                     z: event.position.z as i32 + dz,
    //                 };
    //                 layer.set_block(pos, BlockState::AIR);
    //             }
    //         }
    //     }
    // }

    for event in events.read() {

        let Ok(mut layer) = layers.get_mut(event.layer.0) else {
            continue;
        };
    
        let points = (0..15)
            .flat_map(|x| (0..15).map(move |y| DVec3::new(-1.0 + x as f64 / 8.0, -1.0 + y as f64 / 8.0, 1.0)))
            .chain((0..15).flat_map(|z| (0..15).map(move |y| DVec3::new(1.0, -1.0 + y as f64 / 8.0, -1.0 + z as f64 / 8.0))))
            .chain((0..15).flat_map(|x| (0..15).map(move |y| DVec3::new(-1.0 + x as f64 / 8.0, -1.0 + y as f64 / 8.0, -1.0))))
            .chain((0..15).flat_map(|z| (0..15).map(move |y| DVec3::new(-1.0, -1.0 + y as f64 / 8.0, -1.0 + z as f64 / 8.0))))
            .chain((0..16).flat_map(|z| (0..16).map(move |x| DVec3::new(-1.0 + x as f64 / 8.0, 1.0, -1.0 + z as f64 / 8.0))))
            .chain((1..14).flat_map(|z| (1..14).map(move |x| DVec3::new(-1.0 + x as f64 / 8.0, -1.0, -1.0 + z as f64 / 8.0))))
            .map(|ray| ray.normalize());

        for point in points {
            commands.spawn(
                (
                    RayCaster::new(bevy::math::DVec3::new(event.position.x, event.position.y, event.position.z), Dir3::from_xyz(point.x as f32, point.y as f32, point.z as f32).unwrap()).with_max_time_of_impact(10.0),
                    ExplosionKnockback(4.0),
                )
            );
            let mut dist = 0.0;
            let mut power = 4.0;
            while power > 0.0 {
                let pos = point * dist + event.position;
                if let Some(block) = layer.block([pos.x as i32, pos.y as i32, pos.z as i32]) {
                    if block.state != BlockState::AIR {
                        power -= (0.5 + 0.3) * 0.3;
                        if power > 0.0 {
                            if block.state.to_kind() == BlockKind::Tnt {
                                commands.spawn(TntEntityBundle {
                                    position: Position(pos),
                                    velocity: Velocity((pos - event.position).as_vec3()),
                                    tnt_fuse: Fuse((100.0 / (power + 1.0)) as i32),
                                    layer: event.layer,
                                    ..Default::default()
                                });
                            }
                            layer.set_block([pos.x as i32, pos.y as i32, pos.z as i32], BlockState::AIR);
                            if let Some((collider, _)) = colliders.iter().find(|(_, pos)| pos.x as i32 == event.position.x as i32
                                && pos.y as i32 == event.position.y as i32
                                && pos.z as i32 == event.position.z as i32)
                            {
                                commands.entity(collider).insert(Despawned);
                            }
                        }
                    }
                }
                dist += 0.3;
                power -= 0.22500001;
            }
        }
    }
}

fn knockback(
    hits: Query<(&RayCaster, &RayHits, &ExplosionKnockback)>,
    mut entities: Query<&mut LinearVelocity>,
) {
    for (ray, hits, knockback) in &hits {
        for hit in hits.iter() {
            if let Ok(mut linear) = entities.get_mut(hit.entity) {
                linear.0 += ray.direction.as_dvec3() / (hit.time_of_impact + 1.0) * knockback.0 / 100.0;
            }
        }
    }
}

use avian3d::math::Matrix3;
use bevy::asset::Assets;
use bevy::render::mesh::Mesh;
use bevy::scene::SceneSpawner;
use bevy::transform::bundles::TransformBundle;
use bevy::transform::components::Transform;
use bevy_time::Time;
use valence::entity::falling_block::FallingBlockEntity;
use valence::entity::ObjectData;
use valence::entity::{entity::NoGravity, OnGround, Velocity};
use valence::math::{Aabb, IVec3};
use valence::prelude::*;
use valence::prelude::Position;

use avian3d::prelude::*;

use crate::block_update::BlockUpdateEvent;

pub struct Kinematics;

impl Plugin for Kinematics {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<SceneSpawner>()
            // .insert_resource(Gravity(bevy::math::DVec3::NEG_Y * 100.0))
            .insert_resource(Time::<Physics>::default().with_relative_speed(5.0))
            // .insert_resource(SubstepCount(1000))
            .add_plugins(PhysicsPlugins::new(Update))
            .add_systems(
                Update,
                (
                    generate_colliders,
                    cleanup_colliders,
                    delete_colliders,
                    init_entities,
                    update_entities,
                    (land, reset_on_ground, handle_collisions).chain(),
                ),
            );
    }
}

#[derive(PhysicsLayer)]
enum CollisionLayer {
    Block,
    Entity,
}

fn init_entities(
    entities: Query<(Entity, &Position, &HitboxShape), (Changed<HitboxShape>, Without<Client>)>,
    mut commands: Commands,
) {
    for (entity, position, hitbox) in &entities {
        let (min, max) = (hitbox.get().min(), hitbox.get().max());
        commands.entity(entity).insert((
            Mass(1.0),
            Inertia(Matrix3::IDENTITY),
            RigidBody::Dynamic,
            TransformBundle::from_transform(Transform::from_xyz(position.0.x as f32, position.0.y as f32, position.0.z as f32)),
            Collider::cuboid(max.x - min.x, max.y - min.y, max.z - min.z),
            LockedAxes::new().lock_rotation_x().lock_rotation_y().lock_rotation_z(),
            Restitution::new(0.0),
            CollisionLayers::new(CollisionLayer::Entity, CollisionLayer::Block),
        ));
    }
}

fn update_entities(
    mut entities: Query<(&mut Position, &mut Velocity, &avian3d::position::Position, &LinearVelocity, &Hitbox)>,
) {
    for (mut pos, mut velocity, transform, linear, hitbox) in &mut entities {
        let hitbox_pos = hitbox.min() / 2.0 + hitbox.max() / 2.0 - pos.0;
        pos.0 = DVec3::new(
            transform.0.x - hitbox_pos.x,
            transform.0.y - hitbox_pos.y,
            transform.0.z - hitbox_pos.z,
        );
        velocity.0 = Vec3::new(
            linear.0.x as f32,
            linear.0.y as f32,
            linear.0.z as f32,
        );
    }
}

fn cleanup_colliders(
    colliders: Query<(Entity, &ColliderAabb), Without<Hitbox>>,
    entities: Query<&Position, With<Hitbox>>,
    mut commands: Commands,
) {
    let mut n = 0;
    let mut collider_positions = Vec::new();
    for (entity, collider) in &colliders {
        n += 1;
        let pos = DVec3::new(collider.center().x, collider.center().y, collider.center().z);
        let block_pos = BlockPos {
            x: pos.x as i32,
            y: pos.y as i32,
            z: pos.z as i32,
        };
        if !entities.iter().any(|p| p.distance_squared(pos) < 100.0) || collider_positions.contains(&block_pos) {
            commands.entity(entity).despawn();
        } else {
            collider_positions.push(block_pos);
        }
    }
    tracing::debug!("Collider count: {n}");
}

fn delete_colliders(
    mut events: EventReader<BlockUpdateEvent>,
    layers: Query<&ChunkLayer>,
    colliders: Query<(Entity, &ColliderAabb), Without<Hitbox>>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let Ok(layer) = layers.get(event.layer) {
            if !layer.block(event.position).is_some_and(|b| b.state.collision_shapes().len() > 0) {
                for (collider, _) in colliders.iter().filter(|(_, c)| c.center().x as i32 == event.position.x && c.center().y as i32 == event.position.y && c.center().z as i32 == event.position.z) {
                    commands.entity(collider).despawn();
                }
            }
        }
    }
}

fn generate_colliders(
    entities: Query<(&Position, &OldPosition, &EntityLayerId), (Changed<Position>, With<Hitbox>)>,
    colliders: Query<&ColliderAabb>,
    layers: Query<&ChunkLayer>,
    mut commands: Commands,
) {
    let mut collider_positions = colliders.iter().map(|c| BlockPos {
        x: c.center().x as i32,
        y: c.center().y as i32,
        z: c.center().z as i32,
    }).collect::<Vec<_>>();

    for (pos, old_pos, layer_id) in &entities {
        if pos.x as i32 == old_pos.x as i32
            && pos.y as i32 == old_pos.y as i32
            && pos.z as i32 == old_pos.z as i32
        {
            continue;
        }

        let Ok(layer) = layers.get(layer_id.0) else {
            continue;
        };

        let block_pos = IVec3::new(
            pos.x as i32,
            pos.y as i32,
            pos.z as i32,
        );

        let positions = (-5..=5)
            .flat_map(|y| (-5..=5).flat_map(move |z| (-2..=2).map(move |x| BlockPos { x, y, z })))
            .map(|pos| pos + block_pos)
            .filter(|pos| !collider_positions.contains(pos))
            .collect::<Vec<_>>();

        let blocks = positions
            .iter()
            .filter_map(|pos| layer.block(*pos).map(|b| (b.state, *pos)))
            .flat_map(|(block, pos)| block.collision_shapes().map(move |s| (s, pos)))
            .map(|(shape, pos)| (
                RigidBody::Static,
                Collider::cuboid(
                    shape.max().x - shape.min().x - 0.02,
                    shape.max().y - shape.min().y - 0.02,
                    shape.max().z - shape.min().z - 0.02,
                ),
                Restitution::new(0.0),
                TransformBundle::from_transform(Transform::from_xyz(pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5)),
                CollisionLayers::new(CollisionLayer::Block, CollisionLayer::Entity),
            )).collect::<Vec<_>>();

        commands.spawn_batch(blocks);

        collider_positions.extend(positions);
    }
}

fn handle_collisions(
    mut events: EventReader<Collision>,
    mut entities: Query<&mut OnGround>,
    colliders: Query<(), (With<Collider>, Without<Hitbox>)>,
    mut commands: Commands,
) {
    let mut n = 0;
    for Collision(contacts) in events.read() {
        n += 1;
        if let Ok(mut on_ground) = entities.get_mut(contacts.entity1) {
            on_ground.0 = true;
        } else if let Ok(mut on_ground) = entities.get_mut(contacts.entity2) {
            on_ground.0 = true;
        } else if colliders.get(contacts.entity1).is_ok() && colliders.get(contacts.entity2).is_ok() {
            commands.entity(contacts.entity2).insert(Despawned);
        }
    }
    tracing::debug!("{n} collisions");
}

fn reset_on_ground(mut entities: Query<&mut OnGround>) {
    for mut on_ground in &mut entities {
        on_ground.0 = false;
    }
}

#[allow(dead_code)]
fn ground(
    mut entities: Query<(
        &mut OnGround,
        &mut Position,
        &mut Velocity,
        &OldPosition,
        &EntityLayerId,
        &Hitbox,
    )>,
    layers: Query<&ChunkLayer>,
) {
    for (mut on_ground, mut position, mut velocity, old_position, layer_id, hitbox) in &mut entities
    {
        let Ok(layer) = layers.get(layer_id.0) else {
            continue;
        };

        let colliders = {
            let position = *position;
            (-1..=1)
                .flat_map(move |y| {
                    (-1..=1).flat_map(move |z| {
                        (-1..=1).flat_map(move |x| {
                            let block_pos = BlockPos {
                                x: position.0.x.floor() as i32 + x,
                                y: position.0.y.round() as i32 + y,
                                z: position.0.z.floor() as i32 + z,
                            };
                            layer.block(block_pos).into_iter().flat_map(move |b| {
                                b.state.collision_shapes().map(move |c| {
                                    c + DVec3::new(
                                        position.0.x + x as f64,
                                        position.0.y + y as f64 + 1.0,
                                        position.0.z + z as f64,
                                    )
                                })
                            })
                        })
                    })
                })
                .collect::<Vec<Aabb>>()
        };

        let offset = 0.5 * DVec3::X + 0.5 * DVec3::Z;
        let Some(collides) = colliders.iter().find(|c| {
            c.intersects(hitbox.get() + offset)
                && !c.intersects(
                    hitbox.get() + offset + (old_position.get() - position.0) * DVec3::Y,
                )
        }) else {
            continue;
        };

        // tracing::info!(collides, collided);
        on_ground.0 = true;
        velocity.0.y = 0.0;
        position.0.y = collides.max().y;
    }
}

#[allow(dead_code)]
fn update(mut entities: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut entities {
        position.0 += DVec3::from(velocity.0 / 20.);
    }
}

#[allow(dead_code)]
fn gravity(
    mut entities: Query<(&mut Velocity, &OnGround, &NoGravity, &EntityKind), Without<Client>>,
) {
    for (mut velocity, on_ground, no_gravity, &kind) in &mut entities {
        if no_gravity.0 || on_ground.0 {
            continue;
        }

        let (acceleration, drag, drag_applied_before) = match kind {
            EntityKind::AXOLOTL
            | EntityKind::BAT
            | EntityKind::CAMEL
            | EntityKind::CAT
            | EntityKind::CAVE_SPIDER
            | EntityKind::CHICKEN
            | EntityKind::COD
            | EntityKind::COW
            | EntityKind::CREEPER
            | EntityKind::DOLPHIN
            | EntityKind::DONKEY
            | EntityKind::DROWNED
            | EntityKind::ELDER_GUARDIAN
            | EntityKind::ENDERMAN
            | EntityKind::ENDERMITE
            | EntityKind::EVOKER
            | EntityKind::FOX
            | EntityKind::FROG
            | EntityKind::GHAST
            | EntityKind::GIANT
            | EntityKind::GLOW_SQUID
            | EntityKind::GOAT
            | EntityKind::GUARDIAN
            | EntityKind::HOGLIN
            | EntityKind::HORSE
            | EntityKind::HUSK
            | EntityKind::ILLUSIONER
            | EntityKind::IRON_GOLEM
            | EntityKind::LLAMA
            | EntityKind::MAGMA_CUBE
            | EntityKind::MOOSHROOM
            | EntityKind::MULE
            | EntityKind::OCELOT
            | EntityKind::PANDA
            | EntityKind::PIG
            | EntityKind::PIGLIN
            | EntityKind::PIGLIN_BRUTE
            | EntityKind::PILLAGER
            | EntityKind::POLAR_BEAR
            | EntityKind::PUFFERFISH
            | EntityKind::RABBIT
            | EntityKind::RAVAGER
            | EntityKind::SALMON
            | EntityKind::SHEEP
            | EntityKind::SILVERFISH
            | EntityKind::SKELETON
            | EntityKind::SKELETON_HORSE
            | EntityKind::SLIME
            | EntityKind::SNIFFER
            | EntityKind::SNOW_GOLEM
            | EntityKind::SPIDER
            | EntityKind::SQUID
            | EntityKind::STRAY
            | EntityKind::STRIDER
            | EntityKind::TADPOLE
            | EntityKind::TRADER_LLAMA
            | EntityKind::TROPICAL_FISH
            | EntityKind::TURTLE
            | EntityKind::VILLAGER
            | EntityKind::VINDICATOR
            | EntityKind::WANDERING_TRADER
            | EntityKind::WARDEN
            | EntityKind::WITCH
            | EntityKind::WITHER_SKELETON
            | EntityKind::WOLF
            | EntityKind::ZOGLIN
            | EntityKind::ZOMBIE
            | EntityKind::ZOMBIE_HORSE
            | EntityKind::ZOMBIE_VILLAGER
            | EntityKind::ZOMBIFIED_PIGLIN => (0.08, 0.02, false),
            EntityKind::ITEM | EntityKind::FALLING_BLOCK | EntityKind::TNT => (0.04, 0.02, false),
            EntityKind::MINECART
            | EntityKind::CHEST_MINECART
            | EntityKind::FURNACE_MINECART
            | EntityKind::SPAWNER_MINECART
            | EntityKind::COMMAND_BLOCK_MINECART
            | EntityKind::TNT_MINECART => (0.04, 0.05, false),
            EntityKind::BOAT | EntityKind::CHEST_BOAT => (0.04, 0.0, false),
            EntityKind::EGG | EntityKind::SNOWBALL | EntityKind::ENDER_PEARL => (0.03, 0.01, true),
            EntityKind::POTION => (0.05, 0.01, true),
            EntityKind::EXPERIENCE_BOTTLE => (0.07, 0.01, true),
            EntityKind::EXPERIENCE_ORB => (0.03, 0.02, false),
            EntityKind::FISHING_BOBBER => (0.03, 0.08, false),
            EntityKind::LLAMA_SPIT => (0.06, 0.01, true),
            EntityKind::ARROW | EntityKind::TRIDENT => (0.05, 0.01, true),
            EntityKind::FIREBALL
            | EntityKind::SMALL_FIREBALL
            | EntityKind::WITHER_SKULL
            | EntityKind::DRAGON_FIREBALL => (0.10, 0.05, false),
            _ => (0.0, 0.0, false),
        };

        let new_velocity = if drag_applied_before {
            velocity.0.y * (1. - drag) - acceleration * 20.
        } else {
            (velocity.0.y - acceleration * 20.) * (1. - drag)
        };

        velocity.0.y = new_velocity;
    }
}

fn land(
    mut layers: Query<&mut ChunkLayer>,
    mut falling_blocks: Query<
        (Entity, &Position, &ObjectData, &OnGround, &EntityLayerId),
        With<FallingBlockEntity>,
    >,
    mut commands: Commands,
) {
    for (entity, position, data, on_ground, layer_id) in &mut falling_blocks {
        if !on_ground.0 {
            continue;
        }

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        let Some(block) = BlockState::from_raw(data.0 as u16) else {
            continue;
        };

        commands.entity(entity).insert(Despawned);

        let block_pos = BlockPos {
            x: position.0.x.floor() as i32,
            y: position.0.y.floor() as i32,
            z: position.0.z.floor() as i32,
        };

        layer.set_block(block_pos, block);
    }
}

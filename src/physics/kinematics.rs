use valence::entity::falling_block::FallingBlockEntity;
use valence::entity::ObjectData;
use valence::entity::{entity::NoGravity, OnGround, Velocity};
use valence::math::Aabb;
use valence::prelude::*;
use valence::prelude::Position;

pub struct Kinematics;

impl Plugin for Kinematics {
    fn build(&self, app: &mut App) {
        app.add_systems(
                Update,
                (
                    collide.before(update),
                    update,
                    gravity,
                    drag,
                    land,
                ),
            );
    }
}

fn collide(
    mut entities: Query<(
        &mut OnGround,
        &mut Position,
        &mut Velocity,
        &EntityLayerId,
        &Hitbox,
    ), Without<Client>>,
    layers: Query<&ChunkLayer>,
) {
    for (
        mut on_ground,
        mut position,
        mut velocity,
        layer_id,
        hitbox,
    ) in &mut entities {
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
                                y: position.0.y.floor() as i32 + y,
                                z: position.0.z.floor() as i32 + z,
                            };
                            layer.block(block_pos).into_iter().flat_map(move |b| {
                                b.state.collision_shapes().map(move |c| {
                                    c + DVec3::new(
                                        (position.0.x.floor() as i32 + x) as f64,
                                        (position.0.y.floor() as i32 + y) as f64,
                                        (position.0.z.floor() as i32 + z) as f64,
                                    )
                                })
                            })
                        })
                    })
                })
                .collect::<Vec<Aabb>>()
        };

        const STEP_COUNT: u32 = 8;

        on_ground.0 = false;

        let mut step_offset = DVec3::ZERO;
        let mut new_offset = DVec3::ZERO;
        for _ in 0..STEP_COUNT {
            step_offset += velocity.0.as_dvec3() / (20.0 * STEP_COUNT as f64);
            new_offset = offset(hitbox.get(), &colliders, step_offset);

            if new_offset.x != step_offset.x {
                velocity.x = 0.0;
            }
            if new_offset.y != step_offset.y {
                velocity.y = 0.0;
                on_ground.0 = true;
            }
            if new_offset.z != step_offset.z {
                velocity.z = 0.0;
            }
        }

        position.0 += new_offset;
    }
}

fn offset(a: Aabb, bs: &[Aabb], mut offset: DVec3) -> DVec3 {
    for b in bs {
        if offset.y > 0.0 && (a + offset * DVec3::Y).intersects(*b) {
            offset.y = offset.y.min(b.min().y - a.max().y);
        } else if offset.y < 0.0 && (a + offset * DVec3::Y).intersects(*b) {
            offset.y = offset.y.max(b.max().y - a.min().y);
        }
    }
    for b in bs {
        if offset.x > 0.0 && (a + offset * DVec3::X).intersects(*b) {
            offset.x = offset.x.min(b.min().x - a.max().x);
        } else if offset.x > 0.0 && (a + offset * DVec3::X).intersects(*b) {
            offset.x = offset.x.max(b.max().x - a.min().x);
        }
    }
    for b in bs {
        if offset.z > 0.0 && (a + offset * DVec3::Z).intersects(*b) {
            offset.z = offset.z.min(b.min().z - a.max().z);
        } else if offset.z > 0.0 && (a + offset * DVec3::Z).intersects(*b) {
            offset.z = offset.z.max(b.max().z - a.min().z);
        }
    }
    offset
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
    ), Without<Client>>,
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
                                        position.0.y + y as f64,
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

fn update(mut entities: Query<(&mut Position, &Velocity), Without<Client>>) {
    for (mut position, velocity) in &mut entities {
        position.0 += DVec3::from(velocity.0 / 20.);
    }
}

fn entity_physics_properties(kind: EntityKind, on_ground: bool) -> (f32, f32, f32, bool) {
    match kind {
        EntityKind::ARMOR_STAND
        | EntityKind::AXOLOTL
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
        | EntityKind::ZOMBIFIED_PIGLIN => if on_ground {
            (0.08, 0.02, 0.454, false)
        } else {
            (0.08, 0.02, 0.09, false)
        },
        EntityKind::ITEM | EntityKind::FALLING_BLOCK | EntityKind::TNT => (0.04, 0.02, 0.02, false),
        EntityKind::MINECART
        | EntityKind::CHEST_MINECART
        | EntityKind::FURNACE_MINECART
        | EntityKind::SPAWNER_MINECART
        | EntityKind::COMMAND_BLOCK_MINECART
        | EntityKind::TNT_MINECART => (0.04, 0.05, 0.05, false),
        EntityKind::BOAT | EntityKind::CHEST_BOAT => (0.04, 0.0, 0.10, false),
        EntityKind::EGG | EntityKind::SNOWBALL | EntityKind::ENDER_PEARL => (0.03, 0.01, 0.01, true),
        EntityKind::POTION => (0.05, 0.01, 0.01, true),
        EntityKind::EXPERIENCE_BOTTLE => (0.07, 0.01, 0.01, true),
        EntityKind::EXPERIENCE_ORB => (0.03, 0.02, 0.02, false),
        EntityKind::FISHING_BOBBER => (0.03, 0.08, 0.08, false),
        EntityKind::LLAMA_SPIT => (0.06, 0.01, 0.01, true),
        EntityKind::ARROW | EntityKind::TRIDENT => (0.05, 0.01, 0.01, true),
        EntityKind::FIREBALL
        | EntityKind::SMALL_FIREBALL
        | EntityKind::WITHER_SKULL
        | EntityKind::DRAGON_FIREBALL => (0.10, 0.05, 0.05, false),
        _ => (0.0, 0.0, 0.0, false),
    }
}

fn drag(
    mut entities: Query<(&mut Velocity, &OnGround, &EntityKind), Without<Client>>,
) {
    for (mut velocity, on_ground, &kind) in &mut entities {
        let (_, _, drag, _) = entity_physics_properties(kind, on_ground.0);

        velocity.x *= 1.0 - drag;
        velocity.z *= 1.0 - drag;
    }
}

fn gravity(
    mut entities: Query<(&mut Velocity, &OnGround, &NoGravity, &EntityKind), Without<Client>>,
) {
    for (mut velocity, on_ground, no_gravity, &kind) in &mut entities {
        if no_gravity.0 || on_ground.0 {
            continue;
        }

        let (acceleration, drag, _, drag_applied_before) = entity_physics_properties(kind, on_ground.0);

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

use std::time::Duration;

use bevy_time::{Time, TimePlugin, Timer, TimerMode};
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::layer::chunk::IntoBlock;
use valence::prelude::*;

use crate::block_update::BlockUpdateEvent;

pub struct Fluids;

impl Plugin for Fluids {
    fn build(&self, app: &mut App) {
        app.add_plugins(TimePlugin).add_systems(
            Update,
            (
                placing,
                start_water_flow,
                flowing_water,
                start_lava_flow,
                flowing_lava,
            )
                .chain(),
        );
    }
}

fn placing(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    mut block_updates: EventWriter<BlockUpdateEvent>,
) {
    for event in events.read() {
        let Ok((mut inventory, game_mode, held, &layer_id)) = clients.get_mut(event.client) else {
            continue;
        };

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        if event.head_inside_block {
            continue;
        }

        // get the held item
        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            // no item in the slot
            continue;
        };

        let state = match stack.item {
            ItemKind::WaterBucket => BlockState::WATER,
            ItemKind::LavaBucket => BlockState::LAVA,
            _ => {
                continue;
            }
        };

        let real_pos = event.position.get_in_direction(event.face);

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            inventory.set_slot(slot_id, ItemStack::new(ItemKind::Bucket, 1, None));
        }

        // if let Some(waterloggable) = layer
        //     .block(real_pos)
        //     .filter(|b| b.state.get(PropName::Waterlogged) == Some(PropValue::False))
        // {
        //     state = waterloggable.set(PropName::Waterlogged, PropValue::True);
        // }

        // client.send_chat_message(format!("{:?}", state));
        layer.set_block(real_pos, state);

        block_updates.send(BlockUpdateEvent {
            position: real_pos,
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::Up),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::Down),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::North),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::East),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::South),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::West),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
    }
}

#[derive(Component)]
struct WaterFlowTimer(Timer);

#[derive(Component)]
struct WaterBlockPos(BlockPos);

#[derive(Bundle)]
struct WaterFlowBundle {
    timer: WaterFlowTimer,
    position: WaterBlockPos,
    layer: EntityLayerId,
}

impl WaterFlowBundle {
    fn new(pos: BlockPos, layer: Entity) -> Self {
        Self {
            timer: WaterFlowTimer(Timer::new(Duration::from_millis(250), TimerMode::Once)),
            position: WaterBlockPos(pos),
            layer: EntityLayerId(layer),
        }
    }
}

fn start_water_flow(
    flow_entities: Query<&WaterBlockPos, With<WaterFlowTimer>>,
    layers: Query<&ChunkLayer>,
    mut commands: Commands,
    mut block_updates: EventReader<BlockUpdateEvent>,
) {
    let mut to_spawn = Vec::<WaterFlowBundle>::new();

    for event in block_updates.read() {
        let Ok(layer) = layers.get(event.layer) else {
            continue;
        };

        if !layer
            .block(event.position)
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
        {
            continue;
        }

        if flow_entities.iter().any(|e| e.0 == event.position)
            || to_spawn.iter().any(|b| b.position.0 == event.position)
        {
            continue;
        }

        to_spawn.push(WaterFlowBundle::new(event.position, event.layer));
    }
    commands.spawn_batch(to_spawn);
}

fn flowing_water(
    mut flow_entities: Query<(Entity, &WaterBlockPos, &mut WaterFlowTimer, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    time: Res<Time>,
    mut commands: Commands,
    mut block_updates: EventWriter<BlockUpdateEvent>,
) {
    for (entity, position, mut timer, &layer_id) in &mut flow_entities {
        if !timer.0.tick(time.delta()).finished() {
            continue;
        }

        commands.entity(entity).despawn();

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        let Some(mut block) = layer
            .block(position.0)
            .filter(|b| b.state.to_kind() == BlockKind::Water)
            .map(IntoBlock::into_block)
        else {
            continue;
        };
        let original_state = block.state;

        if !layer
            .block(position.0.get_in_direction(Direction::Up))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
        {
            block.state = block.state.set(
                PropName::Level,
                match block.state.get(PropName::Level) {
                    Some(PropValue::_8) => PropValue::_1,
                    Some(PropValue::_9) => PropValue::_2,
                    Some(PropValue::_10) => PropValue::_3,
                    Some(PropValue::_11) => PropValue::_4,
                    Some(PropValue::_12) => PropValue::_5,
                    Some(PropValue::_13) => PropValue::_6,
                    Some(PropValue::_14) => PropValue::_7,
                    Some(PropValue::_15) | None => {
                        continue;
                    }
                    Some(level) => level,
                },
            );
            layer.set_block(position.0, block.state);
        }

        let adjacent_sources = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .into_iter()
        .filter(|d| {
            layer
                .block(position.0.get_in_direction(*d))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && b.state.get(PropName::Level) == Some(PropValue::_0)
                })
        })
        .count();

        if adjacent_sources >= 2 {
            block.state = block.state.set(
                PropName::Level,
                match block.state.get(PropName::Level) {
                    Some(PropValue::_8) | Some(PropValue::_9) | Some(PropValue::_10)
                    | Some(PropValue::_11) | Some(PropValue::_12) | Some(PropValue::_13)
                    | Some(PropValue::_14) | Some(PropValue::_15) => PropValue::_8,
                    _ => PropValue::_0,
                },
            );
            layer.set_block(position.0, block.state);
        }

        if !layer
            .block(position.0.get_in_direction(Direction::Up))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
            && !layer
                .block(position.0.get_in_direction(Direction::North))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::East))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::South))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::West))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && block.state.get(PropName::Level) != Some(PropValue::_0)
        {
            block.state = if block.state.get(PropName::Level) == Some(PropValue::_7) {
                BlockState::AIR
            } else {
                block.state.set(
                    PropName::Level,
                    match block.state.get(PropName::Level) {
                        Some(PropValue::_0) => PropValue::_1,
                        Some(PropValue::_1) => PropValue::_2,
                        Some(PropValue::_2) => PropValue::_3,
                        Some(PropValue::_3) => PropValue::_4,
                        Some(PropValue::_4) => PropValue::_5,
                        Some(PropValue::_5) => PropValue::_6,
                        Some(PropValue::_6) => PropValue::_7,
                        _ => {
                            continue;
                        }
                    },
                )
            };
            layer.set_block(position.0, block.state);
        }
        if layer
            .block(position.0.get_in_direction(Direction::Down))
            .is_some_and(|b| {
                b.state.is_air()
                    || (b.state.to_kind() == BlockKind::Water
                        && b.state.get(PropName::Level) != Some(PropValue::_0))
            })
        {
            layer.set_block(
                position.0.get_in_direction(Direction::Down),
                block.state.set(
                    PropName::Level,
                    match block.state.get(PropName::Level) {
                        Some(PropValue::_0) | // => PropValue::_8,
                        Some(PropValue::_1) | // => PropValue::_9,
                        Some(PropValue::_2) | // => PropValue::_10,
                        Some(PropValue::_3) | // => PropValue::_11,
                        Some(PropValue::_4) | // => PropValue::_12,
                        Some(PropValue::_5) | // => PropValue::_13,
                        Some(PropValue::_6) | // => PropValue::_14,
                        Some(PropValue::_7) => PropValue::_8,
                        Some(level) => level,
                        _ => {
                            continue;
                        }
                    },
                ),
            );
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::North),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::East),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::South),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::West),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
        } else if !layer
            .block(position.0.get_in_direction(Direction::Down))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water
                        && b.state.get(PropName::Level) != Some(PropValue::_0))
        {
            if layer
                .block(position.0.get_in_direction(Direction::North))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::North),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_15) => {
                                continue;
                            }
                            Some(PropValue::_6) | Some(PropValue::_14) => PropValue::_7,
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_6,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_5,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_4,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_3,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_2,
                            _ => PropValue::_1,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::East))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::East),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_15) => {
                                continue;
                            }
                            Some(PropValue::_6) | Some(PropValue::_14) => PropValue::_7,
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_6,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_5,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_4,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_3,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_2,
                            _ => PropValue::_1,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::South))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::South),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_15) => {
                                continue;
                            }
                            Some(PropValue::_6) | Some(PropValue::_14) => PropValue::_7,
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_6,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_5,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_4,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_3,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_2,
                            _ => PropValue::_1,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::West))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::West),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_15) => {
                                continue;
                            }
                            Some(PropValue::_6) | Some(PropValue::_14) => PropValue::_7,
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_6,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_5,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_4,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_3,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_2,
                            _ => PropValue::_1,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
        }

        if block.state != original_state {
            block_updates.send(BlockUpdateEvent {
                position: position.0,
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Up),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::North),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::East),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::South),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::West),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
        }
    }
}

fn level(state: BlockState) -> Option<u8> {
    match state.get(PropName::Level) {
        Some(PropValue::_0) | Some(PropValue::_8) => Some(0),
        Some(PropValue::_1) | Some(PropValue::_9) => Some(1),
        Some(PropValue::_2) | Some(PropValue::_10) => Some(2),
        Some(PropValue::_3) | Some(PropValue::_11) => Some(3),
        Some(PropValue::_4) | Some(PropValue::_12) => Some(4),
        Some(PropValue::_5) | Some(PropValue::_13) => Some(5),
        Some(PropValue::_6) | Some(PropValue::_14) => Some(6),
        Some(PropValue::_7) | Some(PropValue::_15) => Some(7),
        _ => None,
    }
}

#[derive(Component)]
struct LavaFlowTimer(Timer);

#[derive(Component)]
struct LavaBlockPos(BlockPos);

#[derive(Bundle)]
struct LavaFlowBundle {
    timer: LavaFlowTimer,
    position: LavaBlockPos,
    layer: EntityLayerId,
}

impl LavaFlowBundle {
    fn new(pos: BlockPos, layer: Entity) -> Self {
        Self {
            timer: LavaFlowTimer(Timer::new(Duration::from_millis(1500), TimerMode::Once)),
            position: LavaBlockPos(pos),
            layer: EntityLayerId(layer),
        }
    }
}

fn start_lava_flow(
    flow_entities: Query<&LavaBlockPos, With<LavaFlowTimer>>,
    layers: Query<&ChunkLayer>,
    mut commands: Commands,
    mut block_updates: EventReader<BlockUpdateEvent>,
) {
    let mut to_spawn = Vec::<LavaFlowBundle>::new();

    for event in block_updates.read() {
        let Ok(layer) = layers.get(event.layer) else {
            continue;
        };

        if !layer
            .block(event.position)
            .is_some_and(|b| b.state.to_kind() == BlockKind::Lava)
        {
            continue;
        }

        if flow_entities.iter().any(|e| e.0 == event.position)
            || to_spawn.iter().any(|b| b.position.0 == event.position)
        {
            continue;
        }

        to_spawn.push(LavaFlowBundle::new(event.position, event.layer));
    }
    commands.spawn_batch(to_spawn);
}

fn flowing_lava(
    mut flow_entities: Query<(Entity, &LavaBlockPos, &mut LavaFlowTimer, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    time: Res<Time>,
    mut commands: Commands,
    mut block_updates: EventWriter<BlockUpdateEvent>,
) {
    for (entity, position, mut timer, &layer_id) in &mut flow_entities {
        if !timer.0.tick(time.delta()).finished() {
            continue;
        }

        commands.entity(entity).despawn();

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        let Some(mut block) = layer
            .block(position.0)
            .filter(|b| b.state.to_kind() == BlockKind::Lava)
            .map(IntoBlock::into_block)
        else {
            continue;
        };
        let original_state = block.state;

        if !layer
            .block(position.0.get_in_direction(Direction::Up))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
        {
            block.state = block.state.set(
                PropName::Level,
                match block.state.get(PropName::Level) {
                    Some(PropValue::_8) => PropValue::_2,
                    Some(PropValue::_9) => PropValue::_3,
                    Some(PropValue::_10) => PropValue::_4,
                    Some(PropValue::_11) => PropValue::_5,
                    Some(PropValue::_12) => PropValue::_6,
                    Some(PropValue::_13) => PropValue::_7,
                    Some(PropValue::_14) | Some(PropValue::_15) | None => {
                        continue;
                    }
                    Some(level) => level,
                },
            );
            layer.set_block(position.0, block.state);
        }

        if !layer
            .block(position.0.get_in_direction(Direction::Up))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
            && !layer
                .block(position.0.get_in_direction(Direction::North))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::East))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::South))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && !layer
                .block(position.0.get_in_direction(Direction::West))
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Water
                        && level(b.state)
                            .zip(level(block.state))
                            .is_some_and(|(a, b)| a < b)
                })
            && block.state.get(PropName::Level) != Some(PropValue::_0)
        {
            block.state = if block.state.get(PropName::Level) == Some(PropValue::_6)
                || block.state.get(PropName::Level) == Some(PropValue::_7)
            {
                BlockState::AIR
            } else {
                block.state.set(
                    PropName::Level,
                    match block.state.get(PropName::Level) {
                        Some(PropValue::_0) => PropValue::_2,
                        Some(PropValue::_1) => PropValue::_3,
                        Some(PropValue::_2) => PropValue::_4,
                        Some(PropValue::_3) => PropValue::_5,
                        Some(PropValue::_4) => PropValue::_6,
                        Some(PropValue::_5) => PropValue::_7,
                        _ => {
                            continue;
                        }
                    },
                )
            };
            layer.set_block(position.0, block.state);
        }
        if layer
            .block(position.0.get_in_direction(Direction::Down))
            .is_some_and(|b| {
                b.state.is_air()
                    || (b.state.to_kind() == BlockKind::Water
                        && b.state.get(PropName::Level) != Some(PropValue::_0))
            })
        {
            layer.set_block(
                position.0.get_in_direction(Direction::Down),
                block.state.set(
                    PropName::Level,
                    match block.state.get(PropName::Level) {
                        Some(PropValue::_0) | // => PropValue::_8,
                        Some(PropValue::_1) | // => PropValue::_9,
                        Some(PropValue::_2) | // => PropValue::_10,
                        Some(PropValue::_3) | // => PropValue::_11,
                        Some(PropValue::_4) | // => PropValue::_12,
                        Some(PropValue::_5) | // => PropValue::_13,
                        Some(PropValue::_6) | // => PropValue::_14,
                        Some(PropValue::_7) => PropValue::_8,
                        Some(level) => level,
                        _ => {
                            continue;
                        }
                    },
                ),
            );
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::North),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::East),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::South),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::West),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position
                    .0
                    .get_in_direction(Direction::Down)
                    .get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
        } else if !layer
            .block(position.0.get_in_direction(Direction::Down))
            .is_some_and(|b| b.state.to_kind() == BlockKind::Lava
                        && b.state.get(PropName::Level) != Some(PropValue::_0))
        {
            if layer
                .block(position.0.get_in_direction(Direction::North))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::North),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_6) | Some(PropValue::_15)
                            | Some(PropValue::_14) => {
                                continue;
                            }
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_7,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_6,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_5,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_4,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_3,
                            _ => PropValue::_2,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::North)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::East))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::East),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_6) | Some(PropValue::_15)
                            | Some(PropValue::_14) => {
                                continue;
                            }
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_7,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_6,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_5,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_4,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_3,
                            _ => PropValue::_2,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::East)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::South))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::South),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_6) | Some(PropValue::_15)
                            | Some(PropValue::_14) => {
                                continue;
                            }
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_7,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_6,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_5,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_4,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_3,
                            _ => PropValue::_2,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::East),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::South)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
            if layer
                .block(position.0.get_in_direction(Direction::West))
                .is_some_and(|b| b.state.is_air())
            {
                layer.set_block(
                    position.0.get_in_direction(Direction::West),
                    block.state.set(
                        PropName::Level,
                        match block.state.get(PropName::Level) {
                            Some(PropValue::_7) | Some(PropValue::_6) | Some(PropValue::_15)
                            | Some(PropValue::_14) => {
                                continue;
                            }
                            Some(PropValue::_5) | Some(PropValue::_13) => PropValue::_7,
                            Some(PropValue::_4) | Some(PropValue::_12) => PropValue::_6,
                            Some(PropValue::_3) | Some(PropValue::_11) => PropValue::_5,
                            Some(PropValue::_2) | Some(PropValue::_10) => PropValue::_4,
                            Some(PropValue::_1) | Some(PropValue::_9) => PropValue::_3,
                            _ => PropValue::_2,
                        },
                    ),
                );
                block_updates.send(BlockUpdateEvent {
                    position: position.0.get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::North),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::Up),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::South),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::West),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
                block_updates.send(BlockUpdateEvent {
                    position: position
                        .0
                        .get_in_direction(Direction::West)
                        .get_in_direction(Direction::Down),
                    layer: layer_id.0,
                    entity_layer: layer_id,
                });
            }
        }

        if block.state != original_state {
            block_updates.send(BlockUpdateEvent {
                position: position.0,
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Up),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::North),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::East),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::South),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::West),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: position.0.get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
        }
    }
}


use std::collections::VecDeque;

use valence::{layer::chunk::IntoBlock, prelude::*};

use crate::block_update::BlockUpdateEvent;

pub struct Redstone;

impl Plugin for Redstone {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_block_update);
    }
}

fn handle_block_update(
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<BlockUpdateEvent>,
) {
    let mut queue = VecDeque::new();

    queue.extend(events.read().cloned());

    while let Some(event) = queue.pop_front() {

        let Ok(mut layer) = layers.get_mut(event.layer) else {
            continue;
        };
        let Some(original_block) = layer.block(event.position) else {
            continue;
        };
        let mut block = original_block.into_block();
        let original_block = block.clone();

        let cardinal = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        let surrounding = cardinal.into_iter().map(|d| layer.block(event.position.get_in_direction(d)));
        if block.state.to_kind() == BlockKind::RedstoneWire {
            if surrounding.clone().flatten().any(|b| b.state.to_kind() == BlockKind::RedstoneBlock) {
                block.state = block.state.set(PropName::Power, PropValue::_15);
            } else {
                let max_strength = cardinal.into_iter().filter_map(|d| layer.block(event.position.get_in_direction(d))).filter(|b| b.state.to_kind() == BlockKind::RedstoneWire).filter_map(|b| to_power(b.state)).max().unwrap_or(0);
                block.state = from_power(block.state, max_strength.saturating_sub(1));
            }
        }
        if block != original_block {
            layer.set_block(event.position, block.state);
            
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::Up),
                ..event
            });
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::Down),
                ..event
            });
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::North),
                ..event
            });
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::East),
                ..event
            });
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::South),
                ..event
            });
            queue.push_back(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::West),
                ..event
            });
        }
    }
}

fn to_power(state: BlockState) -> Option<u8> {
    match state.get(PropName::Power) {
        Some(PropValue::_0) => Some(0),
        Some(PropValue::_1) => Some(1),
        Some(PropValue::_2) => Some(2),
        Some(PropValue::_3) => Some(3),
        Some(PropValue::_4) => Some(4),
        Some(PropValue::_5) => Some(5),
        Some(PropValue::_6) => Some(6),
        Some(PropValue::_7) => Some(7),
        Some(PropValue::_8) => Some(8),
        Some(PropValue::_9) => Some(9),
        Some(PropValue::_10) => Some(10),
        Some(PropValue::_11) => Some(11),
        Some(PropValue::_12) => Some(12),
        Some(PropValue::_13) => Some(13),
        Some(PropValue::_14) => Some(14),
        Some(PropValue::_15) => Some(15),
        _ => None,
    }
}

fn from_power(state: BlockState, power: u8) -> BlockState {
    state.set(PropName::Power, match power {
        0 => PropValue::_0,
        1 => PropValue::_1,
        2 => PropValue::_2,
        3 => PropValue::_3,
        4 => PropValue::_4,
        5 => PropValue::_5,
        6 => PropValue::_6,
        7 => PropValue::_7,
        8 => PropValue::_8,
        9 => PropValue::_9,
        10 => PropValue::_10,
        11 => PropValue::_11,
        12 => PropValue::_12,
        13 => PropValue::_13,
        14 => PropValue::_14,
        _ => PropValue::_15,
    })
}

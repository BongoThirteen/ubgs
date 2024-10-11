use std::collections::VecDeque;

use valence::entity::ObjectData;
use valence::entity::falling_block::FallingBlockEntityBundle;
use valence::layer::chunk::IntoBlock;
use valence::prelude::*;

pub struct BlockUpdate;

impl Plugin for BlockUpdate {
    fn build(&self, app: &mut App) {
        app.insert_resource(Events::<BlockUpdateEvent>::default())
            .add_systems(Update, handle_block_update);
    }
}

#[derive(Event, Debug, Copy, Clone)]
pub struct BlockUpdateEvent {
    pub position: BlockPos,
    pub layer: Entity,
    pub entity_layer: EntityLayerId,
}

pub fn handle_block_update(
    mut layers: Query<&mut ChunkLayer>,
    mut commands: Commands,
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
        let (up, down, north, east, south, west) = (
            layer
                .block(event.position.get_in_direction(Direction::Up))
                .map(IntoBlock::into_block),
            layer
                .block(event.position.get_in_direction(Direction::Down))
                .map(IntoBlock::into_block),
            layer
                .block(event.position.get_in_direction(Direction::North))
                .map(IntoBlock::into_block),
            layer
                .block(event.position.get_in_direction(Direction::East))
                .map(IntoBlock::into_block),
            layer
                .block(event.position.get_in_direction(Direction::South))
                .map(IntoBlock::into_block),
            layer
                .block(event.position.get_in_direction(Direction::West))
                .map(IntoBlock::into_block),
        );

        let mut next = Vec::new();

        if block.state.get(PropName::Shape).is_some() && block.state.get(PropName::Half).is_some() {
            let (north, east, south, west) = (
                north.as_ref().and_then(|b| {
                    b.state
                        .get(PropName::Half)
                        .filter(|h| block.state.get(PropName::Half) == Some(*h))
                        .and(north.as_ref().and_then(|b| b.state.get(PropName::Facing)))
                }),
                east.as_ref().and_then(|b| {
                    b.state
                        .get(PropName::Half)
                        .filter(|h| block.state.get(PropName::Half) == Some(*h))
                        .and(east.as_ref().and_then(|b| b.state.get(PropName::Facing)))
                }),
                south.as_ref().and_then(|b| {
                    b.state
                        .get(PropName::Half)
                        .filter(|h| block.state.get(PropName::Half) == Some(*h))
                        .and(south.as_ref().and_then(|b| b.state.get(PropName::Facing)))
                }),
                west.as_ref().and_then(|b| {
                    b.state
                        .get(PropName::Half)
                        .filter(|h| block.state.get(PropName::Half) == Some(*h))
                        .and(west.as_ref().and_then(|b| b.state.get(PropName::Facing)))
                }),
            );
            block.state = block.state.set(
                PropName::Shape,
                match (
                    block.state.get(PropName::Facing).unwrap(),
                    north,
                    east,
                    south,
                    west,
                ) {
                    // (PropValue::North, _, Some(PropValue::North), _, _) |
                    //     (PropValue::North, _, _, _, Some(PropValue::North)) |
                    //     (PropValue::South, _, Some(PropValue::South), _, _) |
                    //     (PropValue::South, _, _, _, Some(PropValue::South)) |
                    //     (PropValue::East, Some(PropValue::East), _, _, _) |
                    //     (PropValue::East, _, _, Some(PropValue::East), _) |
                    //     (PropValue::West, Some(PropValue::West), _, _, _) |
                    //     (PropValue::West, _, _, Some(PropValue::West), _) => PropValue::Straight,
                    (PropValue::North, Some(PropValue::West), _, _, _)
                        if east != Some(PropValue::North) =>
                    {
                        PropValue::OuterLeft
                    }
                    (PropValue::East, _, Some(PropValue::North), _, _)
                        if south != Some(PropValue::East) =>
                    {
                        PropValue::OuterLeft
                    }
                    (PropValue::South, _, _, Some(PropValue::East), _)
                        if west != Some(PropValue::South) =>
                    {
                        PropValue::OuterLeft
                    }
                    (PropValue::West, _, _, _, Some(PropValue::South))
                        if north != Some(PropValue::West) =>
                    {
                        PropValue::OuterLeft
                    }
                    (PropValue::North, Some(PropValue::East), _, _, _)
                        if west != Some(PropValue::North) =>
                    {
                        PropValue::OuterRight
                    }
                    (PropValue::East, _, Some(PropValue::South), _, _)
                        if north != Some(PropValue::East) =>
                    {
                        PropValue::OuterRight
                    }
                    (PropValue::South, _, _, Some(PropValue::West), _)
                        if east != Some(PropValue::South) =>
                    {
                        PropValue::OuterRight
                    }
                    (PropValue::West, _, _, _, Some(PropValue::North))
                        if south != Some(PropValue::West) =>
                    {
                        PropValue::OuterRight
                    }
                    (PropValue::North, _, _, Some(PropValue::West), _)
                        if west != Some(PropValue::North) =>
                    {
                        PropValue::InnerLeft
                    }
                    (PropValue::East, _, _, _, Some(PropValue::North))
                        if north != Some(PropValue::East) =>
                    {
                        PropValue::InnerLeft
                    }
                    (PropValue::South, Some(PropValue::East), _, _, _)
                        if east != Some(PropValue::South) =>
                    {
                        PropValue::InnerLeft
                    }
                    (PropValue::West, _, Some(PropValue::South), _, _)
                        if south != Some(PropValue::West) =>
                    {
                        PropValue::InnerLeft
                    }
                    (PropValue::North, _, _, Some(PropValue::East), _)
                        if east != Some(PropValue::North) =>
                    {
                        PropValue::InnerRight
                    }
                    (PropValue::East, _, _, _, Some(PropValue::South))
                        if south != Some(PropValue::East) =>
                    {
                        PropValue::InnerRight
                    }
                    (PropValue::South, Some(PropValue::West), _, _, _)
                        if west != Some(PropValue::South) =>
                    {
                        PropValue::InnerRight
                    }
                    (PropValue::West, _, Some(PropValue::North), _, _)
                        if north != Some(PropValue::West) =>
                    {
                        PropValue::InnerRight
                    }
                    _ => PropValue::Straight,
                },
            );
        } else if block.state.to_kind() == BlockKind::Grass
            && !down.as_ref().is_some_and(|b| b.state.is_opaque())
        {
            block.state = BlockState::AIR;
            block.nbt = None;
        } else if block.state.get(PropName::North).is_some()
            && block.state.get(PropName::East).is_some()
            && block.state.get(PropName::South).is_some()
            && block.state.get(PropName::West).is_some()
            && block.state.get(PropName::Power).is_none()
        {
            block.state = block.state.set(
                PropName::North,
                if north.as_ref().is_some_and(|b| {
                    fence_type(b.state.to_kind()) == fence_type(block.state.to_kind())
                }) || north
                    .as_ref()
                    .is_some_and(|b| fence_type(b.state.to_kind()) == FenceType::Connect)
                {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
            block.state = block.state.set(
                PropName::East,
                if east.as_ref().is_some_and(|b| {
                    fence_type(b.state.to_kind()) == fence_type(block.state.to_kind())
                }) || east
                    .as_ref()
                    .is_some_and(|b| fence_type(b.state.to_kind()) == FenceType::Connect)
                {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
            block.state = block.state.set(
                PropName::South,
                if south.as_ref().is_some_and(|b| {
                    fence_type(b.state.to_kind()) == fence_type(block.state.to_kind())
                }) || south
                    .as_ref()
                    .is_some_and(|b| fence_type(b.state.to_kind()) == FenceType::Connect)
                {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
            block.state = block.state.set(
                PropName::West,
                if west.as_ref().is_some_and(|b| {
                    fence_type(b.state.to_kind()) == fence_type(block.state.to_kind())
                }) || west
                    .as_ref()
                    .is_some_and(|b| fence_type(b.state.to_kind()) == FenceType::Connect)
                {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
        } else if block.state.get(PropName::InWall).is_some() {
            let in_wall = match block.state.get(PropName::Facing) {
                Some(PropValue::North) | Some(PropValue::South) => {
                    layer
                        .block(event.position.get_in_direction(Direction::East))
                        .as_ref()
                        .is_some_and(|b| b.state.get(PropName::Up).is_some())
                        || layer
                            .block(event.position.get_in_direction(Direction::West))
                            .as_ref()
                            .is_some_and(|b| b.state.get(PropName::Up).is_some())
                }
                _ => {
                    layer
                        .block(event.position.get_in_direction(Direction::North))
                        .as_ref()
                        .is_some_and(|b| b.state.get(PropName::Up).is_some())
                        || layer
                            .block(event.position.get_in_direction(Direction::South))
                            .as_ref()
                            .is_some_and(|b| b.state.get(PropName::Up).is_some())
                }
            };
            block.state = block.state.set(
                PropName::InWall,
                if in_wall {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
        } else if block.state.to_kind() == BlockKind::GrassBlock
            || block.state.to_kind() == BlockKind::Mycelium
            || block.state.to_kind() == BlockKind::Podzol
        {
            let snowy = layer
                .block(event.position.get_in_direction(Direction::Up))
                .as_ref()
                .is_some_and(|b| {
                    b.state.to_kind() == BlockKind::Snow
                        || b.state.to_kind() == BlockKind::SnowBlock
                });
            block.state = block.state.set(
                PropName::Snowy,
                if snowy {
                    PropValue::True
                } else {
                    PropValue::False
                },
            );
        } else if block.state.to_kind() == BlockKind::MelonStem {
            if east
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::Melon)
            {
                block.state =
                    BlockState::ATTACHED_MELON_STEM.set(PropName::Facing, PropValue::East);
            } else if west
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::Melon)
            {
                block.state =
                    BlockState::ATTACHED_MELON_STEM.set(PropName::Facing, PropValue::West);
            } else if north
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::Melon)
            {
                block.state =
                    BlockState::ATTACHED_MELON_STEM.set(PropName::Facing, PropValue::North);
            } else if south
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::Melon)
            {
                block.state =
                    BlockState::ATTACHED_MELON_STEM.set(PropName::Facing, PropValue::South);
            }
        } else if block.state.to_kind() == BlockKind::AttachedMelonStem {
            let dir = match block.state.get(PropName::Facing) {
                Some(PropValue::North) => Direction::North,
                Some(PropValue::East) => Direction::East,
                Some(PropValue::South) => Direction::South,
                _ => Direction::West,
            };
            if layer
                .block(event.position.get_in_direction(dir))
                .filter(|b| b.state.to_kind() == BlockKind::Melon)
                .is_none()
            {
                block.state = BlockState::MELON_STEM;
            }
        } else if block.state.to_kind() == BlockKind::Rail {
            let (north, east, south, west) = (
                layer
                    .block(event.position.get_in_direction(Direction::North))
                    .as_ref()
                    .and_then(|b| b.state.get(PropName::Shape))
                    .as_ref()
                    .is_some_and(|s| {
                        *s == PropValue::NorthSouth
                            || *s == PropValue::EastWest
                            || *s == PropValue::SouthEast
                            || *s == PropValue::SouthWest
                    }),
                layer
                    .block(event.position.get_in_direction(Direction::East))
                    .as_ref()
                    .and_then(|b| b.state.get(PropName::Shape))
                    .as_ref()
                    .is_some_and(|s| {
                        *s == PropValue::EastWest
                            || *s == PropValue::NorthSouth
                            || *s == PropValue::NorthWest
                            || *s == PropValue::SouthWest
                    }),
                layer
                    .block(event.position.get_in_direction(Direction::South))
                    .as_ref()
                    .and_then(|b| b.state.get(PropName::Shape))
                    .as_ref()
                    .is_some_and(|s| {
                        *s == PropValue::NorthSouth
                            || *s == PropValue::EastWest
                            || *s == PropValue::NorthEast
                            || *s == PropValue::NorthWest
                    }),
                layer
                    .block(event.position.get_in_direction(Direction::West))
                    .as_ref()
                    .and_then(|b| b.state.get(PropName::Shape))
                    .as_ref()
                    .is_some_and(|s| {
                        *s == PropValue::EastWest
                            || *s == PropValue::NorthSouth
                            || *s == PropValue::SouthEast
                            || *s == PropValue::NorthEast
                    }),
            );
            let facing = block.state.get(PropName::Shape).unwrap();
            if facing != PropValue::NorthSouth && facing != PropValue::EastWest {
                continue;
            }
            let shape = match (north, east, south, west) {
                (false, false, false, false) => facing,
                (_, false, true, false) | (true, false, _, false) => PropValue::NorthSouth,
                (false, _, false, true) | (false, true, false, _) => PropValue::EastWest,
                (_, true, true, _) => PropValue::SouthEast,
                (_, _, true, true) => PropValue::SouthWest,
                (true, true, _, _) => PropValue::NorthEast,
                _ => PropValue::NorthWest,
            };
            block.state = block.state.set(PropName::Shape, shape);
            if !down.as_ref().is_some_and(|b| b.state.is_opaque()) {
                block.state = BlockState::AIR;
                block.nbt = None;
            }
        } else if block.state.to_kind() == BlockKind::RedstoneWire {
            let (up_north, up_east, up_south, up_west) = (
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Up)
                            .get_in_direction(Direction::North),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Up)
                            .get_in_direction(Direction::East),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Up)
                            .get_in_direction(Direction::South),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Up)
                            .get_in_direction(Direction::West),
                    )
                    .map(IntoBlock::into_block),
            );
            let (down_north, down_east, down_south, down_west) = (
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Down)
                            .get_in_direction(Direction::North),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Down)
                            .get_in_direction(Direction::East),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Down)
                            .get_in_direction(Direction::South),
                    )
                    .map(IntoBlock::into_block),
                layer
                    .block(
                        event
                            .position
                            .get_in_direction(Direction::Down)
                            .get_in_direction(Direction::West),
                    )
                    .map(IntoBlock::into_block),
            );
            if north
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
            {
                block.state = block.state.set(PropName::North, PropValue::Side);
            } else if up_north
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !up.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::North, PropValue::Up);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Up)
                        .get_in_direction(Direction::North),
                );
            } else if down_north
                .clone()
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !north.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::North, PropValue::Side);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Down)
                        .get_in_direction(Direction::North),
                );
            } else {
                block.state = block.state.set(PropName::North, PropValue::None);
            }
            if east
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
            {
                block.state = block.state.set(PropName::East, PropValue::Side);
            } else if up_east
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !up.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::East, PropValue::Up);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Up)
                        .get_in_direction(Direction::East),
                );
            } else if down_east
                .clone()
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !east.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::East, PropValue::Side);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Down)
                        .get_in_direction(Direction::East),
                );
            } else {
                block.state = block.state.set(PropName::East, PropValue::None);
            }
            if south
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
            {
                block.state = block.state.set(PropName::South, PropValue::Side);
            } else if up_south
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !up.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::South, PropValue::Up);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Up)
                        .get_in_direction(Direction::South),
                );
            } else if down_south
                .clone()
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !south.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::South, PropValue::Side);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Down)
                        .get_in_direction(Direction::South),
                );
            } else {
                block.state = block.state.set(PropName::South, PropValue::None);
            }
            if west
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
            {
                block.state = block.state.set(PropName::West, PropValue::Side);
            } else if up_west
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !up.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::West, PropValue::Up);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Up)
                        .get_in_direction(Direction::West),
                );
            } else if down_west
                .clone()
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::RedstoneWire)
                && !west.as_ref().is_some_and(|b| b.state.is_opaque())
            {
                block.state = block.state.set(PropName::West, PropValue::Side);
                next.push(
                    event
                        .position
                        .get_in_direction(Direction::Down)
                        .get_in_direction(Direction::West),
                );
            } else {
                block.state = block.state.set(PropName::West, PropValue::None);
            }
            let (connected_north, connected_east, connected_south, connected_west) = (
                block.state.get(PropName::North) != Some(PropValue::None),
                block.state.get(PropName::East) != Some(PropValue::None),
                block.state.get(PropName::South) != Some(PropValue::None),
                block.state.get(PropName::West) != Some(PropValue::None),
            );
            if !connected_east && !connected_south && !connected_west {
                block.state = block.state.set(PropName::South, PropValue::Side);
            }
            if !connected_north && !connected_south && !connected_west {
                block.state = block.state.set(PropName::West, PropValue::Side);
            }
            if !connected_north && !connected_east && !connected_west {
                block.state = block.state.set(PropName::North, PropValue::Side);
            }
            if !connected_north && !connected_east && !connected_south {
                block.state = block.state.set(PropName::East, PropValue::Side);
            }

            if !down.as_ref().is_some_and(|b| b.state.is_opaque()) {
                block.state = BlockState::AIR;
                block.nbt = None;
            }
        } else if block.state.to_kind() == BlockKind::Scaffolding {
            let bottom = !(down
                .as_ref()
                .is_some_and(|b| b.state.to_kind() == BlockKind::Scaffolding)
                || down.is_some_and(|b| b.state.is_opaque()));
            let distance = if bottom {
                north
                    .as_ref()
                    .and_then(|b| b.state.get(PropName::Distance))
                    .into_iter()
                    .chain(east.as_ref().and_then(|b| b.state.get(PropName::Distance)))
                    .chain(south.as_ref().and_then(|b| b.state.get(PropName::Distance)))
                    .chain(west.as_ref().and_then(|b| b.state.get(PropName::Distance)))
                    .map(|d| match d {
                        PropValue::_0 => 0,
                        PropValue::_1 => 1,
                        PropValue::_2 => 2,
                        PropValue::_3 => 3,
                        PropValue::_4 => 4,
                        PropValue::_5 => 5,
                        PropValue::_6 => 6,
                        _ => 7,
                    })
                    .min()
                    .unwrap_or(7)
            } else {
                0
            };
            block.state = block
                .state
                .set(
                    PropName::Bottom,
                    if bottom {
                        PropValue::True
                    } else {
                        PropValue::False
                    },
                )
                .set(
                    PropName::Distance,
                    match distance {
                        0 => PropValue::_0,
                        1 => PropValue::_1,
                        2 => PropValue::_2,
                        3 => PropValue::_3,
                        4 => PropValue::_4,
                        5 => PropValue::_5,
                        6 => PropValue::_6,
                        _ => PropValue::_7,
                    },
                );
        } else if (block.state.to_kind() == BlockKind::Sand
            || block.state.to_kind() == BlockKind::RedSand
            || block.state.to_kind() == BlockKind::Gravel
            || block.state.to_kind() == BlockKind::DragonEgg
            || block.state.to_kind() == BlockKind::Anvil
            || block.state.to_kind() == BlockKind::ChippedAnvil
            || block.state.to_kind() == BlockKind::DamagedAnvil
            || block.state.to_kind() == BlockKind::SuspiciousSand
            || block.state.to_kind() == BlockKind::SuspiciousGravel
            || block.state.to_kind() == BlockKind::WhiteConcretePowder
            || block.state.to_kind() == BlockKind::LightGrayConcretePowder
            || block.state.to_kind() == BlockKind::GrayConcretePowder
            || block.state.to_kind() == BlockKind::BlackConcretePowder
            || block.state.to_kind() == BlockKind::BrownConcretePowder
            || block.state.to_kind() == BlockKind::RedConcretePowder
            || block.state.to_kind() == BlockKind::OrangeConcretePowder
            || block.state.to_kind() == BlockKind::YellowConcretePowder
            || block.state.to_kind() == BlockKind::LimeConcretePowder
            || block.state.to_kind() == BlockKind::GreenConcretePowder
            || block.state.to_kind() == BlockKind::CyanConcretePowder
            || block.state.to_kind() == BlockKind::LightBlueConcretePowder
            || block.state.to_kind() == BlockKind::BlueConcretePowder
            || block.state.to_kind() == BlockKind::PurpleConcretePowder
            || block.state.to_kind() == BlockKind::MagentaConcretePowder
            || block.state.to_kind() == BlockKind::PinkConcretePowder)
            && !down.is_some_and(|b| !b.state.is_air() && !b.state.is_liquid())
        {
            block.state = BlockState::AIR;
            block.nbt = None;
            let position = Position(DVec3::new(f64::from(event.position.x) + 0.5, event.position.y.into(), f64::from(event.position.z) + 0.5));
            commands.spawn(FallingBlockEntityBundle {
                position,
                layer: event.entity_layer,
                object_data: ObjectData(original_block.state.to_raw() as i32),
                ..Default::default()
            });
        }

        if block != original_block {
            queue.extend(next.into_iter().map(|p| BlockUpdateEvent { position: p, ..event }));

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

        layer.set_block(event.position, block);
    }
}

#[derive(PartialEq, Eq)]
enum FenceType {
    Wood,
    Nether,
    Wall,
    Glass,
    Connect,
    None,
}

fn fence_type(block_kind: BlockKind) -> FenceType {
    match block_kind {
        BlockKind::NetherBrickFence => FenceType::Nether,
        BlockKind::OakFence
        | BlockKind::SpruceFence
        | BlockKind::BirchFence
        | BlockKind::JungleFence
        | BlockKind::AcaciaFence
        | BlockKind::DarkOakFence
        | BlockKind::MangroveFence
        | BlockKind::CherryFence
        | BlockKind::BambooFence
        | BlockKind::CrimsonFence
        | BlockKind::WarpedFence => FenceType::Wood,
        BlockKind::CobblestoneWall
        | BlockKind::MossyCobblestoneWall
        | BlockKind::StoneBrickWall
        | BlockKind::MossyStoneBrickWall
        | BlockKind::GraniteWall
        | BlockKind::DioriteWall
        | BlockKind::AndesiteWall
        | BlockKind::CobbledDeepslateWall
        | BlockKind::PolishedDeepslateWall
        | BlockKind::DeepslateBrickWall
        | BlockKind::DeepslateTileWall
        | BlockKind::BrickWall
        | BlockKind::MudBrickWall
        | BlockKind::SandstoneWall
        | BlockKind::RedSandstoneWall
        | BlockKind::PrismarineWall
        | BlockKind::NetherBrickWall
        | BlockKind::RedNetherBrickWall
        | BlockKind::BlackstoneWall
        | BlockKind::PolishedBlackstoneWall
        | BlockKind::PolishedBlackstoneBrickWall
        | BlockKind::EndStoneBrickWall => FenceType::Wall,
        BlockKind::GlassPane
        | BlockKind::WhiteStainedGlassPane
        | BlockKind::LightGrayStainedGlassPane
        | BlockKind::GrayStainedGlassPane
        | BlockKind::BlackStainedGlassPane
        | BlockKind::BrownStainedGlassPane
        | BlockKind::RedStainedGlassPane
        | BlockKind::OrangeStainedGlassPane
        | BlockKind::YellowStainedGlassPane
        | BlockKind::LimeStainedGlassPane
        | BlockKind::GreenStainedGlassPane
        | BlockKind::CyanStainedGlassPane
        | BlockKind::LightBlueStainedGlassPane
        | BlockKind::BlueStainedGlassPane
        | BlockKind::PurpleStainedGlassPane
        | BlockKind::MagentaStainedGlassPane
        | BlockKind::PinkStainedGlassPane
        | BlockKind::IronBars => FenceType::Glass,
        _ if block_kind.to_state().is_opaque() => FenceType::Connect,
        _ => FenceType::None,
    }
}

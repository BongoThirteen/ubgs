
use std::time::Instant;

use valence::command::handler::CommandResultEvent;
use valence::command::parsers::{CommandArg, CommandArgParseError, ParseInput, Vec3};
use valence::command::AddCommand;
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::message::SendMessage;
use valence::nbt::{compound, Value};
use valence::prelude::*;
use valence::command_macros::Command;
use valence::protocol::packets::play::command_tree_s2c::Parser;

use crate::building::{CancelDiggingEvent, digging};

pub struct Edit;

impl Plugin for Edit {
    fn build(&self, app: &mut App) {
        app
            .add_command::<Pos1Command>()
            .add_command::<Pos2Command>()
            .add_command::<SetCommand>()
            .add_command::<WandCommand>()
            .add_systems(
                Update,
                (
                    handle_pos1_command,
                    handle_pos2_command,
                    handle_set_command,
                    handle_wand_command,
                    handle_wand_break.before(digging),
                    handle_wand_use,
                ),
            );
    }
}

#[derive(Component)]
struct RectArea {
    min: Option<BlockPos>,
    max: Option<BlockPos>,
}

#[derive(Command)]
#[paths("pos1 {position?}", "p1 {position?}")]
#[scopes("valence.command.edit.select")]
struct Pos1Command {
    position: Option<Vec3>,
}

fn handle_pos1_command(
    mut events: EventReader<CommandResultEvent<Pos1Command>>,
    mut clients: Query<(&mut Client, &Position)>,
    mut areas: Query<&mut RectArea>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut client, position)) = clients.get_mut(event.executor) else {
            continue;
        };

        let position = match event.result.position {
            None => BlockPos {
                x: position.0.x as i32,
                y: position.0.y as i32,
                z: position.0.z as i32,
            },
            Some(rel_pos) => BlockPos {
                x: rel_pos.x.get(position.x as f32) as i32,
                y: rel_pos.y.get(position.y as f32) as i32,
                z: rel_pos.z.get(position.z as f32) as i32,
            }
        };

        if let Ok(mut area) = areas.get_mut(event.executor) {
            area.min = Some(position);
        } else {
            commands.entity(event.executor).insert(RectArea { min: Some(position), max: None });
        }

        client.send_chat_message(format!("Set first position to block at ({}, {}, {})", position.x, position.y, position.z).color(Color::DARK_AQUA));
    }
}

#[derive(Command)]
#[paths("pos2 {position?}", "p2 {position?}")]
#[scopes("valence.command.edit.select")]
struct Pos2Command {
    position: Option<Vec3>,
}

fn handle_pos2_command(
    mut events: EventReader<CommandResultEvent<Pos2Command>>,
    mut clients: Query<(&mut Client, &Position)>,
    mut areas: Query<&mut RectArea>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut client, position)) = clients.get_mut(event.executor) else {
            continue;
        };

        let position = match event.result.position {
            None => BlockPos {
                x: position.0.x as i32,
                y: position.0.y as i32,
                z: position.0.z as i32,
            },
            Some(rel_pos) => BlockPos {
                x: rel_pos.x.get(position.x as f32) as i32,
                y: rel_pos.y.get(position.y as f32) as i32,
                z: rel_pos.z.get(position.z as f32) as i32,
            }
        };

        if let Ok(mut area) = areas.get_mut(event.executor) {
            area.max = Some(position);
        } else {
            commands.entity(event.executor).insert(RectArea { min: None, max: Some(position) });
        }

        client.send_chat_message(format!("Set second position to block at ({}, {}, {})", position.x, position.y, position.z).color(Color::DARK_AQUA));
    }
}

struct ParseBlockState(BlockState);

impl CommandArg for ParseBlockState {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let string = String::parse_arg(input)?;
        let (kind_name, states) = match string.split_once('[') {
            Some((kind_name, states)) => (kind_name, states.trim_end_matches(']')),
            None => (&string[..], ""),
        };
        let kind = BlockKind::from_str(kind_name.trim_start_matches("minecraft:")).ok_or_else(|| CommandArgParseError::InvalidArgument { expected: "block".into(), got: string.clone() })?;
        let mut state = BlockState::from_kind(kind);
        for (name, value) in states.split(',').filter_map(|state| state.split_once('=')) {
            state = state.set(
                PropName::from_str(name).ok_or_else(|| CommandArgParseError::InvalidArgument { expected: "block property".into(), got: name.into() })?,
                PropValue::from_str(value).ok_or_else(|| CommandArgParseError::InvalidArgument { expected: "block property value".into(), got: value.into() })?,
            );
        }

        Ok(Self(state))
    }
    fn display() -> Parser {
        Parser::BlockState
    }
}

#[derive(Command)]
#[paths("set {block}", "s {block}")]
#[scopes("valence.command.edit.set")]
struct SetCommand {
    block: ParseBlockState,
}

fn handle_set_command(
    mut events: EventReader<CommandResultEvent<SetCommand>>,
    mut clients: Query<(&EntityLayerId, &mut Client)>,
    areas: Query<&RectArea>,
    mut layers: Query<&mut ChunkLayer>,
) {
    for event in events.read() {
        let Ok((layer_id, mut client)) = clients.get_mut(event.executor) else {
            continue;
        };

        let Ok(RectArea { min: Some(min), max: Some(max) }) = areas.get(event.executor) else {
            client.send_chat_message("You must fully select an area before setting blocks. Use the /pos1 and /pos2 commands or the /wand to set both corners.".color(Color::RED));
            continue;
        };

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            client.send_chat_message("Internal error: chunk layer not found".color(Color::RED));
            continue;
        };

        let (min_x, max_x) = if max.x >= min.x {
            (min.x, max.x + 1)
        } else {
            (max.x, min.x + 1)
        };
        let (min_y, max_y) = if max.y >= min.y {
            (min.y, max.y + 1)
        } else {
            (max.y, min.y + 1)
        };
        let (min_z, max_z) = if max.z >= min.z {
            (min.z, max.z + 1)
        } else {
            (max.z, min.z + 1)
        };

        let block_count = (max_x - min_x) * (max_y - min_y) * (max_z - min_z);
        client.send_chat_message(format!("Setting {block_count} blocks...").color(Color::DARK_AQUA));

        let start_time = Instant::now();

        for y in min_y..max_y {
            for z in min_z..max_z {
                for x in min_x..max_x {
                    layer.set_block(BlockPos { x, y, z}, event.result.block.0);
                }
            }
        }

        let time = Instant::now() - start_time;

        client.send_chat_message(format!("Successfully set {block_count} blocks in {time:?}").color(Color::GREEN));
    }
}

#[derive(Command, Debug, Clone)]
#[paths("wand", "w")]
#[scopes("valence.command.edit.wand")]
struct WandCommand;

fn handle_wand_command(
    mut events: EventReader<CommandResultEvent<WandCommand>>,
    mut clients: Query<(&mut Client, &mut Inventory, &HeldItem)>,
) {
    for event in events.read() {
        let Ok((mut client, mut inventory, held)) = clients.get_mut(event.executor) else {
            continue;
        };

        let nbt = Some(compound! {
            "wand" => "rect",
        });
        if inventory.slot(held.slot()).is_empty() {
            inventory.set_slot(held.slot(), ItemStack { item: ItemKind::Stick, count: 1, nbt });
        } else {
            let Some(slot) = inventory.first_empty_slot_in(0..9) else {
                client.send_chat_message("There are no free spaces in your inventory".color(Color::RED));
                continue;
            };
            inventory.set_slot(slot, ItemStack { item: ItemKind::Stick, count: 1, nbt });
        }

        client.send_chat_message("Use this stick to select blocks".color(Color::DARK_AQUA));
    }
}

fn handle_wand_use(
    mut events: EventReader<InteractBlockEvent>,
    mut clients: Query<(&mut Client, &Inventory, &HeldItem)>,
    mut areas: Query<&mut RectArea>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut client, inventory, held)) = clients.get_mut(event.client) else {
            continue;
        };

        if !inventory.slot(held.slot()).nbt.as_ref().is_some_and(|nbt| nbt.get("wand") == Some(&Value::String("rect".into()))) {
            continue;
        }

        if let Ok(mut area) = areas.get_mut(event.client) {
            if area.max == Some(event.position) {
                continue;
            }
            area.max = Some(event.position);
        } else {
            commands.entity(event.client).insert(RectArea { min: None, max: Some(event.position) });
        }

        client.send_chat_message(format!("Set second position to block at ({}, {}, {})", event.position.x, event.position.y, event.position.z).color(Color::DARK_AQUA));
    }
}

fn handle_wand_break(
    mut events: EventReader<DiggingEvent>,
    mut cancel: EventWriter<CancelDiggingEvent>,
    mut clients: Query<(&mut Client, &Inventory, &HeldItem)>,
    mut areas: Query<&mut RectArea>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut client, inventory, held)) = clients.get_mut(event.client) else {
            continue;
        };

        if !inventory.slot(held.slot()).nbt.as_ref().is_some_and(|nbt| nbt.get("wand") == Some(&Value::String("rect".into()))) {
            continue;
        }

        if let Ok(mut area) = areas.get_mut(event.client) {
            if area.min == Some(event.position) {
                continue;
            }
            area.min = Some(event.position);
        } else {
            commands.entity(event.client).insert(RectArea { min: Some(event.position), max: None });
        }

        cancel.send(CancelDiggingEvent { client: event.client });

        client.send_chat_message(format!("Set first position to block at ({}, {}, {})", event.position.x, event.position.y, event.position.z).color(Color::DARK_AQUA));
    }
}

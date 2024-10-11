use std::borrow::Cow;
use valence::client::DisconnectClient;
use valence::protocol::anyhow;
use valence::abilities::PlayerAbilitiesFlags;
use valence::entity::player::PlayerEntityBundle;
use valence::message::SendMessage;
use valence::protocol::packets::play::DisconnectS2c;
use valence::protocol::WritePacket;
use valence::inventory::HeldItem;
use valence::prelude::*;

use crate::SPAWN_POS;
use crate::anvil::{autosave, send_recv_chunks, AnvilLevel};
use crate::exit::handle_exit;

pub struct Players;

impl Plugin for Players {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerLoadEvent>()
            .add_systems(
            Update,
            (handle_loaded_clients, init_clients.after(handle_loaded_clients), despawn_disconnected_clients, save_players.after(send_recv_chunks)),
        ).add_systems(Update, disconnect_on_shutdown.after(handle_exit).before(autosave));
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Xp {
    pub level: i32,
    pub bar: f32,
}

#[derive(Debug)]
pub struct PlayerData {
    pub entity: PlayerEntityBundle,
    pub inventory: Inventory,
    pub game_mode: GameMode,
    pub held_item: u8,
    pub flying: bool,
    pub xp: Xp,
    pub dimension: Ident<String>,
}

#[derive(Event, Debug)]
pub struct PlayerLoadEvent {
    pub uuid: UniqueId,
    pub data: anyhow::Result<Option<PlayerData>>,
}

fn init_clients(
    server: Res<Server>,
    mut clients: Query<
        (
            Entity,
            &UniqueId,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    mut layers: Query<(Entity, &ChunkLayer, &mut AnvilLevel)>,
    mut commands: Commands,
) {
    let Some((layer, _, mut overworld)) = layers.iter_mut().find(|(_, l, _)| l.dimension_type_name() == ident!("overworld")) else {
        for (client, ..) in &clients {
            commands.add(DisconnectClient {
                client,
                reason: "Could not find `overworld` dimension.".color(Color::RED),
            });
        }
        return;
    };

    for (
        _client,
        &uuid,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut inventory,
    ) in &mut clients {
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        *inventory = Inventory::new(InventoryKind::Player);
        let _ = overworld.get_player_data(uuid);
        tracing::info!(tick=server.current_tick())
    }
}

fn handle_loaded_clients(
    server: Res<Server>,
    uuids: Query<(Entity, &UniqueId), With<Client>>,
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut Look,
            &mut Inventory,
            &mut HeldItem,
            &mut GameMode,
            &mut PlayerAbilitiesFlags,
            &Username,
        ),
    >,
    mut events: EventReader<PlayerLoadEvent>,
    layers: Query<(Entity, &ChunkLayer)>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Some((entity, _)) = uuids
            .iter()
            .find(|(_, uuid)| **uuid == event.uuid) else {
                tracing::warn!("couldn't find uuid");
                continue;
        };

        let Ok(
            (
                entity,
                mut client,
                mut layer_id,
                mut visible_chunk_layer,
                mut visible_entity_layers,
                mut pos,
                mut look,
                mut inventory,
                mut held_item,
                mut game_mode,
                mut flags,
                _username,
            )
        ) = clients.get_mut(entity) else {
            tracing::warn!("couldn't find client");
            continue;
        };

        // let spawn_pos = (0..384)
        //     .rev()
        //     .map(|y| DVec3::new(SPAWN_POS.x, y as f64, SPAWN_POS.y))
        //     .find(|&pos| chunks.block(pos).is_some_and(|b| b.state == BlockState::AIR))
        //     .unwrap();
        
        match event.data {
            Ok(Some(ref saved)) => {
                let Some((layer, _)) = layers.iter().find(|(_, l)| l.dimension_type_name() == saved.dimension) else {
                    commands.add(DisconnectClient {
                        client: entity,
                        reason: format!("Error: could not find `{}` dimension", saved.dimension.as_str()).color(Color::RED),
                    });
                    continue;
                };
                pos.set(saved.entity.position.0);
                *look = saved.entity.look;
                for slot in 0..saved.inventory.slot_count() {
                    inventory.set_slot(slot, saved.inventory.slot(slot).clone());
                }
                held_item.set_hotbar_idx(saved.held_item);
                layer_id.0 = layer;
                visible_chunk_layer.0 = layer;
                visible_entity_layers.0.insert(layer);
                *game_mode = saved.game_mode;
                flags.set_flying(saved.flying);
                *look = saved.entity.look;
                commands.entity(entity).insert(saved.xp);
                tracing::info!(tick=server.current_tick())
            }
            Ok(None) => {
                let Some((layer, _)) = layers.iter().find(|(_, l)| l.dimension_type_name() == ident!("overworld")) else {
                    commands.add(DisconnectClient {
                        client: entity,
                        reason: "Error: could not find `overworld` dimension".color(Color::RED),
                    });
                    continue;
                };
                pos.set(SPAWN_POS);
                layer_id.0 = layer;
                visible_chunk_layer.0 = layer;
                visible_entity_layers.0.insert(layer);
                *game_mode = GameMode::Creative;
                commands.entity(entity).insert(Xp { level: 0, bar: 0. });
        
                let rust_text = "Rust"
                    .color(Color::AQUA)
                    .underlined()
                    .on_click_open_url("https://rust-lang.org")
                    .on_hover_show_text("https://rust-lang.org".color(Color::AQUA));
                let bevy_text = "Bevy ECS"
                    .color(Color::AQUA)
                    .underlined()
                    .on_click_open_url("https://bevyengine.org")
                    .on_hover_show_text("https://bevyengine.org".color(Color::AQUA));
                let valence_text = "Valence"
                    .color(Color::AQUA)
                    .underlined()
                    .on_click_open_url("https://valence.rs")
                    .on_hover_show_text("https://valence.rs".color(Color::AQUA));
                let message = "Welcome to BongoThirteen's experimental server!\n\n".color(Color::AQUA)
                    + "This server is written from scratch in ".color(Color::DARK_AQUA) + rust_text + " and contains no code from Mojang. It is in an early stage of development, so expect lots of bugs.\n\n".color(Color::DARK_AQUA)
                    + "Thank you to all the projects that make this server possible, including but not limited to:\n\n".color(Color::DARK_AQUA)
                    + "    • ".color(Color::DARK_AQUA) + bevy_text
                    + "\n    • ".color(Color::DARK_AQUA) + valence_text
                    + "\n";
        
                client.send_chat_message(message);
            }
            Err(ref err) => {
                tracing::warn!("failed to load player data: {err:?}");
                let Some((layer, _)) = layers.iter().find(|(_, l)| l.dimension_type_name() == ident!("overworld")) else {
                    commands.add(DisconnectClient {
                        client: entity,
                        reason: "Error: could not find `overworld` dimension".color(Color::RED),
                    });
                    continue;
                };
                pos.set(SPAWN_POS);
                layer_id.0 = layer;
                visible_chunk_layer.0 = layer;
                visible_entity_layers.0.insert(layer);
                *game_mode = GameMode::Creative;
                
                client.send_chat_message("Unfortunately, we couldn't load your player data from the world save due to corruption. Please notify the server administration of this issue.".color(Color::RED));
            }
        }

        // entity_layer.send_chat_message(username.0.clone().color(Color::DARK_AQUA) + " joined the game");
    }
}

fn save_players(
    mut disconnected_clients: RemovedComponents<Client>,
    players: Query<
        (
            &EntityLayerId,
            &UniqueId,
            &Position,
            &Look,
            &Inventory,
            &HeldItem,
            &Xp,
            &GameMode,
            &PlayerAbilitiesFlags,
        )
    >,
    mut layers: Query<(&mut AnvilLevel, &ChunkLayer)>,
) {
    for disconnected_client in disconnected_clients.read() {
        let Ok(
            (
                layer_id,
                uuid,
                position,
                look,
                inventory,
                held,
                xp,
                game_mode,
                flags,
            )
        ) = players.get(disconnected_client) else {
            continue;
        };

        let Ok((mut anvil, layer)) = layers.get_mut(layer_id.0) else {
            continue;
        };

        let dimension = layer.dimension_type_name().to_string_ident();

        let data = PlayerData {
            inventory: inventory.clone(),
            game_mode: *game_mode,
            held_item: held.hotbar_idx(),
            flying: flags.flying(),
            xp: *xp,
            dimension,
            entity: PlayerEntityBundle {
                look: *look,
                position: *position,
                uuid: *uuid,
                ..Default::default()
            },
        };

        anvil.save_player_data(data);
    }
}

// fn save_player_data(
//     mut disconnected_clients: RemovedComponents<Client>,
//     players: Query<(&UniqueId, &Position, &Look, &Inventory, &HeldItem)>,
//     mut player_data: ResMut<PlayerData>,
// ) {
//     for disconnected_client in disconnected_clients.read() {
//         let Ok((uuid, &Position(position), &look, inventory, &held_item)) =
//             players.get(disconnected_client)
//         else {
//             continue;
//         };
//         player_data.0.insert(
//             *uuid,
//             Player {
//                 position,
//                 look,
//                 inventory: inventory.clone(),
//                 held_item,
//             },
//         );
//     }
// }

fn disconnect_on_shutdown(
    mut events: EventReader<AppExit>,
    mut clients: Query<&mut Client>,
) {
    for event in events.read() {
        for mut conn in clients.iter_mut() {
            let msg = match event {
                AppExit::Success => "The server closed".color(Color::DARK_AQUA),
                AppExit::Error(_) => "The server crashed".color(Color::DARK_RED),
            };

            conn.write_packet(&DisconnectS2c { reason: Cow::Owned(msg) });
        }
    }
}

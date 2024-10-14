use valence::entity::living::Health;
use valence::entity::{EntityStatuses, Velocity};
use valence::inventory::HeldItem;
use valence::math::Vec3Swizzles;
use valence::prelude::*;

use crate::death::DeathEvent;

pub struct Combat;

impl Plugin for Combat {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (init_clients, handle_combat_event));
    }
}

#[derive(Component, Default, Debug, Copy, Clone)]
struct CombatState {
    last_attacked_tick: i64,
    has_bonus_knockback: bool,
}

fn init_clients(mut entities: Query<Entity, Added<Health>>, mut commands: Commands) {
    for entity in &mut entities {
        commands.entity(entity).insert(CombatState::default());
    }
}

fn handle_combat_event(
    server: Res<Server>,
    mut clients: Query<(
        Entity,
        &mut Client,
        &Position,
        &Velocity,
        &mut CombatState,
        &mut EntityStatuses,
        &mut Health,
        &mut Inventory,
        &HeldItem,
        &GameMode,
        &Username,
    )>,
    mut entities: Query<(Entity, &Position, &mut bevy_rapier3d::dynamics::Velocity, &mut Health, &mut CombatState), Without<Client>>,
    mut sprinting: EventReader<SprintEvent>,
    mut interact_entity: EventReader<InteractEntityEvent>,
    mut death: EventWriter<DeathEvent>,
) {
    for &SprintEvent { client, state } in sprinting.read() {
        if let Ok((_, _, _, _, mut combat_state, ..)) = clients.get_mut(client) {
            combat_state.has_bonus_knockback = state == SprintState::Start;
        }
    }

    for &InteractEntityEvent {
        client: attacker_entity,
        entity: victim_entity,
        interact: kind,
        ..
    } in interact_entity.read()
    {
        if kind != EntityInteraction::Attack {
            continue;
        }

        let Ok(
            (
                attacker_entity,
                _,
                &attacker_pos,
                _,
                &attacker_state,
                ..,
                inventory,
                held,
                _,
                _,
            )
        ) = clients.get(attacker_entity) else {
            continue;
        };

        let held_item = inventory.slot(held.slot());

        let (damage, _attack_speed) = match held_item.item {
            ItemKind::WoodenSword => (4.0, 1.6),
            ItemKind::GoldenSword => (4.0, 1.6),
            ItemKind::StoneSword => (5.0, 1.6),
            ItemKind::IronSword => (6.0, 1.6),
            ItemKind::DiamondSword => (7.0, 1.6),
            ItemKind::NetheriteSword => (8.0, 1.6),

            ItemKind::Trident => (9.0, 1.1),

            ItemKind::WoodenShovel => (2.5, 1.0),
            ItemKind::GoldenShovel => (2.5, 1.0),
            ItemKind::StoneShovel => (3.5, 1.0),
            ItemKind::IronShovel => (4.5, 1.0),
            ItemKind::DiamondShovel => (5.5, 1.0),
            ItemKind::NetheriteShovel => (6.5, 1.0),

            ItemKind::WoodenPickaxe => (2.0, 1.2),
            ItemKind::GoldenPickaxe => (2.0, 1.2),
            ItemKind::StonePickaxe => (3.0, 1.2),
            ItemKind::IronPickaxe => (4.0, 1.2),
            ItemKind::DiamondPickaxe => (5.0, 1.2),
            ItemKind::NetheritePickaxe => (6.0, 1.2),

            ItemKind::WoodenAxe => (7.0, 0.8),
            ItemKind::GoldenAxe => (7.0, 1.0),
            ItemKind::StoneAxe => (9.0, 0.8),
            ItemKind::IronAxe => (9.0, 0.9),
            ItemKind::DiamondAxe => (9.0, 1.0),
            ItemKind::NetheriteAxe => (10.0, 1.0),

            ItemKind::WoodenHoe => (1.0, 1.0),
            ItemKind::GoldenHoe => (1.0, 1.0),
            ItemKind::StoneHoe => (1.0, 2.0),
            ItemKind::IronHoe => (1.0, 3.0),
            ItemKind::DiamondHoe => (1.0, 4.0),
            ItemKind::NetheriteHoe => (1.0, 4.0),

            _ => (1.0, 4.0),
        };

        if let Ok((entity, mut client, pos, velocity, mut state, mut statuses, mut health, .., game_mode, _)) = clients.get_mut(victim_entity) {
            if *game_mode == GameMode::Creative || *game_mode == GameMode::Spectator {
                continue;
            }

            if server.current_tick() - state.last_attacked_tick < 10 {
                continue;
            }

            state.last_attacked_tick = server.current_tick();

            let dir = (pos.0.xz() - attacker_pos.0.xz())
                .normalize()
                .as_vec2();

            let knockback_xz = if attacker_state.has_bonus_knockback {
                18.0
            } else {
                8.0
            };

            let knockback_y = if attacker_state.has_bonus_knockback {
                8.432
            } else {
                6.432
            };

            client.set_velocity(velocity.0 + Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz));

            client.trigger_status(EntityStatus::PlayAttackSound);
            statuses.trigger(EntityStatus::PlayAttackSound);

            if health.0 > damage {
                health.0 -= damage;
            } else {
                health.0 = 0.0;
                death.send(DeathEvent {
                    entity,
                    killed_by: Some(attacker_entity),
                });
            }
        } else if let Ok((entity, pos, mut velocity, mut health, mut state)) = entities.get_mut(victim_entity) {
            if server.current_tick() - state.last_attacked_tick < 10 {
                continue;
            }

            state.last_attacked_tick = server.current_tick();

            let dir = (pos.0.xz() - attacker_pos.0.xz())
                .normalize().as_vec2();

            let knockback_xz = if attacker_state.has_bonus_knockback {
                18.0
            } else {
                8.0
            };

            let knockback_y = if attacker_state.has_bonus_knockback {
                8.432
            } else {
                6.432
            };

            velocity.linvel += bevy::math::Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz);

            if health.0 > damage {
                health.0 -= damage;
            } else {
                health.0 = 0.0;
                death.send(DeathEvent {
                    entity,
                    killed_by: Some(attacker_entity),
                });
            }
        }

        let Ok(
            (
                _,
                _,
                _,
                _,
                mut attacker_state,
                ..,
                _inventory,
                _held,
                _attacker_game_mode,
            )
        ) = clients.get_mut(attacker_entity) else {
            continue;
        };

        attacker_state.has_bonus_knockback = false;
    }
}

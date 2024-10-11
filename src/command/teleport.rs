
use valence::command::handler::CommandResultEvent;
use valence::command::scopes::CommandScopes;
use valence::command::{AddCommand, CommandScopeRegistry};
use valence::command_macros::Command;
use valence::entity::living::LivingEntity;
use valence::message::SendMessage;
use valence::op_level::OpLevel;
use valence::prelude::*;
use valence::command::parsers::{EntitySelector, Vec3};
use valence::scoreboard::{Objective, ObjectiveScores};

use crate::command::find_targets;
use crate::players::Xp;

pub struct Teleport;

impl Plugin for Teleport {
    fn build(&self, app: &mut App) {
        app
            .add_command::<TeleportCommand>()
            .add_systems(Startup, setup)
            .add_systems(Update, (init_clients, handle_teleport_command));
    }
}

fn setup(
    mut command_scopes: ResMut<CommandScopeRegistry>,
) {
    command_scopes.link("valence.admin", "valence.command");
}

fn init_clients(
    mut clients: Query<(&mut CommandScopes, &mut OpLevel), Added<Client>>,
) {
    for (mut permissions, mut op_level) in &mut clients {
        op_level.set(4);
        permissions.add("valence.admin");
    }
}

#[derive(Command, Debug, Clone)]
#[paths("teleport", "tp")]
#[scopes("valence.command.teleport")]
enum TeleportCommand {
    #[paths = "{location}"]
    ExecutorToLocation { location: Vec3 },
    #[paths = "{target}"]
    ExecutorToTarget { target: EntitySelector },
    #[paths = "{from} {to}"]
    TargetToTarget {
        from: EntitySelector,
        to: EntitySelector,
    },
    #[paths = "{target} {location}"]
    TargetToLocation {
        target: EntitySelector,
        location: Vec3,
    },
}

#[derive(Debug)]
enum TeleportTarget {
    Targets(Vec<Entity>),
}

#[derive(Debug)]
enum TeleportDestination {
    Location(Vec3),
    Target(Option<Entity>),
}

fn handle_teleport_command(
    mut events: EventReader<CommandResultEvent<TeleportCommand>>,
    mut players: Query<
        (
            Entity,
            &EntityLayerId,
            &mut Position,
            &Username,
            &Xp,
            &GameMode,
        )
    >,
    mut living_entities: Query<(Entity, &EntityLayerId, &Position, &EntityKind), (With<LivingEntity>, Without<Username>)>,
    scoreboard: Query<(&Objective, &ObjectiveScores)>,
    mut clients: Query<(Entity, &mut Client)>,
) {
    for event in events.read() {

        let compiled_command = match &event.result {
            TeleportCommand::ExecutorToLocation { location } => Ok((
                TeleportTarget::Targets(vec![event.executor]),
                TeleportDestination::Location(*location),
            )),
            TeleportCommand::ExecutorToTarget { target } => {
                find_targets(
                        &players.transmute_lens().query(),
                        &living_entities.transmute_lens_filtered().query(),
                        &scoreboard,
                        event,
                        target,
                    )
                    .map(|targets| (
                TeleportTarget::Targets(vec![event.executor]),
                TeleportDestination::Target(targets.into_iter().next())))
            }
            TeleportCommand::TargetToTarget { from, to } => {
                find_targets(
                    &players.transmute_lens().query(),
                    &living_entities.transmute_lens_filtered().query(),
                    &scoreboard,
                    event,
                    from,
                ).map(TeleportTarget::Targets).and_then(|targets| {
                find_targets(
                    &players.transmute_lens().query(),
                    &living_entities.transmute_lens_filtered().query(),
                    &scoreboard,
                    event,
                    to,
                ).map(|destinations| (targets, TeleportDestination::Target(destinations.into_iter().next()))) })
            }
            TeleportCommand::TargetToLocation { target, location } => {
                find_targets(
                    &players.transmute_lens().query(),
                    &living_entities.transmute_lens_filtered().query(),
                    &scoreboard,
                    event,
                    target,
                ).map(|targets| (TeleportTarget::Targets(targets),
                TeleportDestination::Location(*location)))
            }
        };

        let (targets, destination) = match compiled_command {
            Ok((TeleportTarget::Targets(targets), destination)) => (targets, destination),
            Err(err) => {
                if let Ok((_, mut client)) = clients.get_mut(event.executor) {
                    client.send_chat_message(err.to_string().color(Color::RED));
                }
                return;
            }
        };

        match destination {
            TeleportDestination::Target(target) => {
                if let Some(target) = target {
                    let Ok((_, _, &target_pos, ..)) = players.get(target) else {
                        if let Ok((_, mut client)) = clients.get_mut(event.executor) {
                            client.send_chat_message("Internal error: target position not found".color(Color::RED));
                        }
                        return;
                    };
                    for target in targets {
                        if let Ok((_, _, mut position, ..)) = players.get_mut(target) {
                            position.0 = target_pos.0;
                        }
                    }
                }
            }
            TeleportDestination::Location(location) => {
                for target in targets {
                    if let Ok((_, _, mut position, ..)) = players.get_mut(target) {
                        position.0.x = location.x.get(position.0.x as f32).into();
                        position.0.y = location.y.get(position.0.y as f32).into();
                        position.0.z = location.z.get(position.0.z as f32).into();
                    }
                }
            }
        }
    }
}


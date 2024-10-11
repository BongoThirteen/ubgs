use valence::client::DisconnectClient;
use valence::command::handler::CommandResultEvent;
use valence::command::parsers::{EntitySelector, GreedyString};
use valence::command_macros::Command;
use valence::entity::living::LivingEntity;
use valence::message::SendMessage;
use valence::player_list::DisplayName;
use valence::scoreboard::{Objective, ObjectiveScores};
use valence::{command::AddCommand, prelude::*};

use crate::players::Xp;

use super::find_targets;

pub struct Kick;

impl Plugin for Kick {
    fn build(&self, app: &mut App) {
        app.add_command::<KickCommand>()
            .add_systems(Update, handle_kick_command);
    }
}

#[derive(Command, Debug, Clone)]
#[paths("kick {target} {reason?}")]
#[scopes("valence.command.kick")]
struct KickCommand {
    target: EntitySelector,
    reason: Option<GreedyString>,
}

fn handle_kick_command(
    mut events: EventReader<CommandResultEvent<KickCommand>>,
    mut clients: Query<(&mut Client, &Username, &DisplayName)>,
    players: Query<
        (
            Entity,
            &EntityLayerId,
            &Position,
            &Username,
            &Xp,
            &GameMode,
        ),
    >,
    living_entities: Query<(Entity, &EntityLayerId, &Position, &EntityKind), With<LivingEntity>>,
    scoreboard: Query<(&Objective, &ObjectiveScores)>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut client, user, display)) = clients.get_mut(event.executor) else {
            continue;
        };
        let targets = match find_targets(
            &players,
            &living_entities,
            &scoreboard,
            event,
            &event.result.target,
        ) {
            Ok(targets) => targets,
            Err(err) => {
                client.send_chat_message(err.to_string().color(Color::RED));
                continue;
            }
        };
        for target in targets {
            commands.add(DisconnectClient {
                client: target,
                reason: "Kicked by ".color(Color::DARK_AQUA)
                    + display.0.clone().unwrap_or(user.0.clone().color(Color::DARK_AQUA))
                    + event
                        .result
                        .reason
                        .clone()
                        .map(|r| "\n\n".color(Color::AQUA) + r.0)
                        .unwrap_or("".into()),
            });
        }
    }
}

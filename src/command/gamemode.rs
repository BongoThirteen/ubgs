
use valence::command::handler::CommandResultEvent;
use valence::command_macros::Command;
use valence::command::{parsers::EntitySelector, AddCommand};
use valence::entity::living::LivingEntity;
use valence::prelude::*;
use valence::scoreboard::{Objective, ObjectiveScores};

use crate::players::Xp;

use super::find_targets;

pub struct Gamemode;

impl Plugin for Gamemode {
    fn build(&self, app: &mut App) {
        app
            .add_command::<GamemodeCommand>()
            .add_systems(Update, handle_gamemode_command);
    }
}

#[derive(Command, Debug, Clone)]
#[paths("gamemode {gamemode} {target?}")]
#[scopes("valence.command.gamemode")]
struct GamemodeCommand {
    gamemode: GameMode,
    target: Option<EntitySelector>,
}

fn handle_gamemode_command(
    mut events: EventReader<CommandResultEvent<GamemodeCommand>>,
    mut clients: Query<(&mut Client, &Username, Entity)>,
    mut players: Query<
        (
            Entity,
            &EntityLayerId,
            &Position,
            &Username,
            &Xp,
            &mut GameMode,
        )
    >,
    scoreboard: Query<(&Objective, &ObjectiveScores)>,
    living_entities: Query<(Entity, &EntityLayerId, &Position, &EntityKind), With<LivingEntity>>,
) {
    for event in events.read() {
        let targets = match &event.result.target {
            None => vec![event.executor],
            Some(selector) => match find_targets(
                &players.transmute_lens().query(),
                &living_entities,
                &scoreboard,
                event,
                selector,
            ) {
                Ok(targets) => targets,
                Err(err) => {
                    if let Ok((mut client, _, _)) = clients.get_mut(event.executor) {
                        client.send_chat_message(err.to_string().color(Color::RED));
                    }
                    return;
                }
            }
        };

        for target in targets {
            if let Ok((.., mut game_mode)) = players.get_mut(target) {
                *game_mode = event.result.gamemode;
            }
        }
    }
}

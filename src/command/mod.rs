
use valence::prelude::*;

mod find;
mod teleport;
mod gamemode;
mod edit;
mod kick;

pub use find::find_targets;
use teleport::Teleport;
use gamemode::Gamemode;
use edit::Edit;
use kick::Kick;

pub struct Command;

impl Plugin for Command {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(Teleport)
            .add_plugins(Gamemode)
            .add_plugins(Edit)
            .add_plugins(Kick);
    }
}

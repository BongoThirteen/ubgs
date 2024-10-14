#![allow(clippy::type_complexity)]

pub mod building;
pub mod block_update;
pub mod players;
pub mod terrain;
pub mod save;
pub mod anvil;
pub mod perf;
pub mod exit;
pub mod server_list;
pub mod explosion;
pub mod physics;
pub mod command;
pub mod combat;
pub mod death;

pub const SPAWN_POS: DVec3 = DVec3::new(0., 70., 0.);

use valence::prelude::*;

pub struct Vanilla;

pub use valence;

impl Plugin for Vanilla {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            (
                server_list::ServerList,
                DefaultPlugins,
                building::Building,
                players::Players,
                exit::Exit,
                explosion::Explosion,
                physics::Physics,
                command::Command,
                combat::Combat,
                death::Death,
            )
        );
    }
}

pub mod prelude {
    use super::*;

    pub use valence::prelude::*;

    pub use super::Vanilla;

    pub use building::Building;
    pub use players::Players;
    pub use exit::Exit;
    pub use server_list::ServerList;
    pub use explosion::Explosion;
    pub use physics::Physics;
    pub use command::Command;
    pub use combat::Combat;
    pub use death::Death;
}

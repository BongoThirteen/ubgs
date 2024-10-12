#![allow(clippy::type_complexity)]

mod building;
mod block_update;
mod players;
mod terrain;
mod save;
mod anvil;
mod perf;
mod exit;
mod server_list;
mod explosion;
mod physics;
mod command;
mod combat;
mod death;
mod worldgen;

use building::Building;
use players::Players;
use exit::Exit;
use server_list::ServerList;
use explosion::Explosion;
use physics::Physics;
use command::Command;
use combat::Combat;
use death::Death;
use worldgen::WorldGen;

use valence::prelude::*;

const SPAWN_POS: DVec3 = DVec3::new(0., 70., 0.);

pub fn main() {
    App::new()
        .add_plugins(Building)
        .add_plugins(Players)
        .add_plugins(ServerList)
        .add_plugins(Explosion)
        .add_plugins(Physics)
        .add_plugins(DefaultPlugins)
        .add_plugins(Command)
        .add_plugins(Combat)
        .add_plugins(Death)
        .add_plugins(WorldGen)
        .add_plugins(Exit)
        .run();
}

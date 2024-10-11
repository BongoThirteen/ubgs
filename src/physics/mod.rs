
use valence::prelude::*;

pub mod kinematics;
pub mod fluids;
pub mod redstone;

use kinematics::Kinematics;
use fluids::Fluids;
use redstone::Redstone;

pub struct Physics;

impl Plugin for Physics {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(Kinematics)
            .add_plugins(Fluids)
            .add_plugins(Redstone);
    }
}

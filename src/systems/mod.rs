mod camera;
mod chassis_observation;
mod debug;
mod debug_window;
mod frequency;
mod input;
mod projectile;
mod range_control;
mod spawn_selection;
mod telemetry_window;
mod uav;
pub use camera::*;
pub use chassis_observation::*;
pub use debug::*;
pub use debug_window::*;
pub use frequency::*;
pub use input::*;
pub use projectile::*;
pub use range_control::*;
pub use spawn_selection::*;
pub use telemetry_window::*;
pub use uav::*;

use bevy::prelude::*;

#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub enum GameplaySystems {
    Input,
    GameLogic,
    Camera,
    Cleanup,
}

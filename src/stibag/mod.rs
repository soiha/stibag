
mod map;
pub mod core;

use bevy::app::App;
use bevy::prelude::*;

pub struct StibagGamePlugin {

}

impl Plugin for StibagGamePlugin {
    fn build(&self, app: &mut App) {
        let mut world = core::World::init();  // TODO this goes in as a resource
        let p_actor = world.spawn_actor_from_template("player".to_string());
        world.player_possess_actor(p_actor);
        world.tick();
    }
}
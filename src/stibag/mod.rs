
mod map;
pub mod core;

use bevy::app::App;
use bevy::prelude::*;

#[derive(Resource)]
pub struct StibagWorldRes {
    world: core::World,
}

#[derive(Resource)]
pub struct StibagRes {
    rect_mesh: Option<Handle<Mesh>>,
}

pub struct StibagGamePlugin {

}

fn plugin_init(mut commands: Commands, asset_server: Res<AssetServer>, st_world: Res<StibagWorldRes>, mut meshes: ResMut<Assets<Mesh>>, mut stibagres: ResMut<StibagRes>) {
    
    stibagres.rect_mesh = Some(meshes.add(Rectangle::new(1.0, 1.0)));
    info!("Stibag plugin init");
}

impl Plugin for StibagGamePlugin {
    fn build(&self, app: &mut App) {
        let mut world = core::World::init(); 
        let p_actor = world.spawn_actor_from_template("player".to_string());
        world.player_possess_actor(p_actor);
        world.tick();
        app.insert_resource(StibagWorldRes {
            world,
        });
        app.insert_resource(StibagRes {
            rect_mesh: None,
        });
        app.add_systems(Startup, plugin_init);
    }
}
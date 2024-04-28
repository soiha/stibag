mod map;
pub mod core;

use bevy::input::gamepad::GamepadConnection;
use bevy::input::gamepad::GamepadEvent;
use bevy::app::App;
use bevy::math::vec3;
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;

const TILE_SIZE: f32 = 10.0;

#[derive(Resource)]
pub struct StibagWorldRes {
    world: core::World,
}

#[derive(Resource)]
pub struct StibagRes {
    rect_mesh: Option<Handle<Mesh>>,
}

#[derive(Component)]
pub struct PlayerMarker;

#[derive(Component)]
pub struct CameraMarker;

#[derive(Bundle)]
struct PlayerBundle {
    player_marker: PlayerMarker,
}

#[derive(Event)]
struct PlayerMovementEvent(IVec2);

#[derive(Resource)]
struct StibagGamepad(Gamepad);

pub struct StibagGamePlugin {}

fn tile_coords_to_world_coords(tile_pos: IVec2) -> Vec2 {
    Vec2::new(tile_pos.x as f32 * TILE_SIZE, tile_pos.y as f32 * TILE_SIZE)
}

fn plugin_init(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>, st_world: Res<StibagWorldRes>, mut meshes: ResMut<Assets<Mesh>>, mut stibagres: ResMut<StibagRes>) {
    stibagres.rect_mesh = Some(meshes.add(Rectangle::new(1.0, 1.0)));
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        }, CameraMarker {},
    ));

    let base_offset = vec3(-00.0, -0.0, 0.0);
    for x in 0..st_world.world.map.width {
        for y in 0..st_world.world.map.height {
            let mut transform = Transform::from_translation(base_offset + Vec3::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE, 1.0));
            let tile = st_world.world.map.get_tile_at(IVec2::new(x as i32, y as i32)).unwrap();

            transform.scale = Vec3::new(TILE_SIZE - 1.0, TILE_SIZE - 1.0, 1.0);
            commands.spawn(MaterialMesh2dBundle {
                mesh: bevy::sprite::Mesh2dHandle(stibagres.rect_mesh.clone().unwrap()),
                material: materials.add(tile.get_color()),
                transform: transform,
                ..default()
            }, );

            // commands.entity(plr.id()).insert(PlayerMarker { });
        }
    }
    let mut player_trans = Transform::from_translation(base_offset + Vec3::new(10.0, 10.0, 5.0));
    player_trans.scale = Vec3::new(5.0, 5.0, 1.0);
    let plr = commands.spawn((MaterialMesh2dBundle {
        mesh: bevy::sprite::Mesh2dHandle(stibagres.rect_mesh.clone().unwrap()),
        material: materials.add(Color::rgb(0.0, 0.6, 1.0)),
        transform: player_trans,
        ..default()
    }, PlayerMarker {}));


    info!("Stibag plugin init");
}

fn player_movement_sys(mut plr_set: ParamSet<(Query<&mut Transform, With<PlayerMarker>>, )>,
                       mut ev_movement: EventReader<PlayerMovementEvent>) {
    let mut p = plr_set.p0();
    let mut plr_transform = p.single_mut();
    let mut pos = plr_transform.translation;
    for ev in ev_movement.read() {
        pos.x += ev.0.x as f32 * TILE_SIZE;
        pos.y += ev.0.y as f32 * TILE_SIZE;
        plr_transform.translation = pos;
        // TODO geo check
    }
}

fn camera_recenter_sys(mut cam_set: ParamSet<(Query<&mut Transform, (With<CameraMarker>, Without<PlayerMarker>)>, )>,
                       mut plr_set: ParamSet<(Query<&Transform, With<PlayerMarker>>, )>) {
    let mut c = cam_set.p0();
    let mut cam_trans = c.single_mut();
    let p = plr_set.p0();
    let plr_trans = p.single();

    cam_trans.translation = plr_trans.translation;
}

fn gamepad_input_events(mut commands: Commands, stibag_gamepad: Option<Res<StibagGamepad>>, mut gamepad_evr: EventReader<GamepadEvent>, mut ev_movement: EventWriter<PlayerMovementEvent>) {
    if let Some(gamepad) = stibag_gamepad {
        for ev in gamepad_evr.read() {
            match ev {
                GamepadEvent::Button(input) => {
                    if (input.gamepad.id == gamepad.0.id && input.value > 0.0) {
                        info!("Button event: {:?}", input);
                        match input.button_type {
                            GamepadButtonType::DPadUp => {
                                ev_movement.send(PlayerMovementEvent(IVec2::new(0, 1)));
                            }
                            GamepadButtonType::DPadDown => {
                                ev_movement.send(PlayerMovementEvent(IVec2::new(0, -1)));
                            }
                            GamepadButtonType::DPadLeft => {
                                ev_movement.send(PlayerMovementEvent(IVec2::new(-1, 0)));
                            }
                            GamepadButtonType::DPadRight => {
                                ev_movement.send(PlayerMovementEvent(IVec2::new(1, 0)));
                            }
                            _ => {}
                        }
                    }
                }
                GamepadEvent::Axis(input) => {
                    info!("Axis event: {:?}", input);
                }
                _ => {}
            }
        }
    }
}

fn gamepad_connections(mut commands: Commands, stibag_gamepad: Option<Res<StibagGamepad>>, mut gamepad_evr: EventReader<GamepadEvent>) {
    for ev in gamepad_evr.read() {
        match ev {
            GamepadEvent::Connection(conn_ev) => {
                match &conn_ev.connection {
                    GamepadConnection::Connected(info) => {
                        info!("Gamepad connected: {:?} ({})", conn_ev.gamepad.id, info.name);
                        if stibag_gamepad.is_none() {
                            commands.insert_resource(StibagGamepad(conn_ev.gamepad));
                        }
                    }
                    GamepadConnection::Disconnected => {
                        info!("Gamepad disconnected: {:?}", conn_ev.gamepad.id);
                        if stibag_gamepad.is_some() {
                            commands.remove_resource::<StibagGamepad>();
                        }
                    }
                }
            }
            _ => {}
        }
    }
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
        app.add_event::<PlayerMovementEvent>();
        app.add_systems(Startup, plugin_init);
        app.add_systems(Update, gamepad_connections);
        app.add_systems(Update, gamepad_input_events);
        app.add_systems(Update, player_movement_sys);
        app.add_systems(Update, camera_recenter_sys);
    }
}
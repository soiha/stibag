mod map;
pub mod core;

use bevy::a11y::accesskit::ListStyle::Square;
use bevy::input::gamepad::GamepadConnection;
use bevy::input::gamepad::GamepadEvent;
use bevy::app::App;
use bevy::math::vec3;
use bevy::prelude::*;
use bevy::reflect::ReflectKind::Map;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_ecs_tilemap::prelude::*;
use stopwatch::Stopwatch;

const TILE_SIZE: f32 = 32.0;

#[derive(Resource)]
pub struct StibagWorldRes {
    world: core::World,
}

#[derive(Resource)]
pub struct StibagRes {
    BLACK: Handle<ColorMaterial>,
    rect_mesh: Option<Handle<Mesh>>,
}

#[derive(Reflect, Component)]
pub struct MapTile {
    pub map_position: IVec2,
    pub color: Color,
    pub in_vision: bool,
    pub is_seen: bool,
}

#[derive(Component)]
pub struct InVisionMarker;

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

fn plugin_init(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>,
               st_world: Res<StibagWorldRes>, mut meshes: ResMut<Assets<Mesh>>, mut stibagres: ResMut<StibagRes>,
               // array_texture_loader?
) {
    stibagres.rect_mesh = Some(meshes.add(Rectangle::new(1.0, 1.0)));
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        }, CameraMarker {},
    ));

    let BLACK = materials.add(Color::rgb(0.0, 0.0, 0.0));
    stibagres.BLACK = BLACK;

    let base_offset = vec3(-00.0, -0.0, 0.0);
    /*
   
    for x in 0..st_world.world.map.width {
        for y in 0..st_world.world.map.height {
            let mut transform = Transform::from_translation(base_offset + Vec3::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE, 1.0));
            let tile = st_world.world.map.get_tile_at(IVec2::new(x as i32, y as i32)).unwrap();

            transform.scale = Vec3::new(TILE_SIZE - 1.0, TILE_SIZE - 1.0, 1.0);
            let mesh = stibagres.rect_mesh.clone().unwrap();
            commands.spawn((MaterialMesh2dBundle {
                mesh: bevy::sprite::Mesh2dHandle(mesh),
                material: materials.add(Color::WHITE),
                transform: transform,
                ..default()
            }, MapTile {
                map_position: IVec2::new(x as i32, y as i32),
                color: tile.get_color(),
                in_vision: false,
                is_seen: false,
            }));

            // commands.entity(plr.id()).insert(PlayerMarker { });
        }
    }
    
     */
    let mut player_trans = Transform::from_translation(base_offset + Vec3::new(10.0, 10.0, 5.0));
    player_trans.scale = Vec3::new(5.0, 5.0, 1.0);
    let plr = commands.spawn((MaterialMesh2dBundle {
        mesh: bevy::sprite::Mesh2dHandle(stibagres.rect_mesh.clone().unwrap()),
        material: materials.add(Color::rgb(0.0, 0.6, 1.0)),
        transform: player_trans,
        ..default()
    }, PlayerMarker {}));

    let texture_handle = asset_server.load("u5_tiles.png");
    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };


    let tmap_size = TilemapSize { x: st_world.world.map.width, y: st_world.world.map.height };
    let mut tile_storage = TileStorage::empty(tmap_size);
    let tmap_type = TilemapType::Square;
    let tmap_entity = commands.spawn_empty().id();

    for x in 0..tmap_size.x {
        for y in 0..tmap_size.y {
            let tile = st_world.world.map.get_tile_at(IVec2::new(x as i32, y as i32)).unwrap();
            let tile_pos = TilePos { x, y };
            let tile_entity = commands.spawn((TileBundle {
                position: tile_pos,
                tilemap_id: TilemapId(tmap_entity),
                texture_index: tile.get_texture_index(),
                ..Default::default()
            })).id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();
    commands.entity(tmap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: tmap_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&tmap_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });

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
                       mut plr_set: ParamSet<(Query<&Transform, With<PlayerMarker>>, )>,
                       mut tilemap_q: ParamSet<(Query<(&Transform, &TilemapType, &TilemapGridSize, &TileStorage), (With<TileStorage>, Without<PlayerMarker>, Without<CameraMarker>)>, )>, ) {
    let tmq = tilemap_q.p0();
    let (map_transform, map_type, grid_size, tilemap_storage) = tmq.single();
    let mut c = cam_set.p0();
    let mut cam_trans = c.single_mut();
    let p = plr_set.p0();
    let plr_trans = p.single();
    let map_pos_v3 = (plr_trans.translation / TILE_SIZE).floor();
    let plr_map_pos = IVec2::new(map_pos_v3.x as i32, map_pos_v3.y as i32);
    let tpos = TilePos { x: plr_map_pos.x as u32, y: plr_map_pos.y as u32 };
    let t_center = tpos.center_in_world(&TilemapGridSize { x: 32.0, y: 32.0 }, map_type).extend(1.0);
    cam_trans.translation = map_transform.translation + t_center;
}

fn recalc_vision_sys(mut commands: Commands, stibagres: Res<StibagRes>, st_world: Res<StibagWorldRes>, mut materials: ResMut<Assets<ColorMaterial>>,
                     mut query: Query<(Entity, &mut MapTile, &mut Handle<ColorMaterial>)>,
                     mut plr_set: ParamSet<(Query<&Transform, With<PlayerMarker>>, )>) {
    let p = plr_set.p0();
    let plr_trans = p.single();
    let map_pos_v3 = (plr_trans.translation / TILE_SIZE).floor();
    let plr_map_pos = IVec2::new(map_pos_v3.x as i32, map_pos_v3.y as i32);
    let sw = Stopwatch::start_new();
    let plr_vision = st_world.world.map.calc_vision(plr_map_pos, 50.0);
    // let plr_vision = vec![plr_map_pos + IVec2::new(0, 1), plr_map_pos + IVec2::new(1, 0), plr_map_pos + IVec2::new(-1, 0), plr_map_pos + IVec2::new(0, -1)];

    for (mut e, mut tile, c_mat) in query.iter_mut() {
        let wt = st_world.world.map.get_tile_at(tile.map_position).unwrap();
        let color_mat = materials.get_mut(c_mat.as_ref()).unwrap();
        if plr_vision.contains(&tile.map_position) {
            tile.in_vision = true;
            tile.is_seen = true;
            commands.entity(e).insert(InVisionMarker);
        } else {
            tile.in_vision = false;
        }
        color_mat.color = if tile.in_vision {
            wt.get_color()
        } else {
            Color::BLACK
        }
    }
    info!("Vision recalc from {} in {} ms vision has {} entries", plr_map_pos, sw.elapsed_ms(), plr_vision.len());
}

fn reassign_vision_markers_sys(mut commands: Commands, st_world: Res<StibagWorldRes>, mut current_viz_query: Query<(Entity, ), With<InVisionMarker>>,
                               mut map_tile_query: Query<(Entity, &TilePos, &mut TileColor)>, mut plr_set: ParamSet<(Query<&Transform, With<PlayerMarker>>, )>, ) {
    let p = plr_set.p0();
    let plr_trans = p.single();
    let map_pos_v3 = (plr_trans.translation / TILE_SIZE).floor();
    let plr_map_pos = IVec2::new(map_pos_v3.x as i32, map_pos_v3.y as i32);
    let sw = Stopwatch::start_new();
    let plr_vision = st_world.world.map.calc_vision(plr_map_pos, 50.0);
    for (e, ) in current_viz_query.iter_mut() {
        commands.entity(e).remove::<InVisionMarker>();
    }
    for (e, tile_pos, tile_color) in map_tile_query.iter() {
        if plr_vision.contains(&IVec2::new(tile_pos.x as i32, tile_pos.y as i32)) {
            commands.entity(e).insert(InVisionMarker);
        }
    }
}

fn set_material_colors_sys(mut commands: Commands, st_world: Res<StibagWorldRes>, mut materials: ResMut<Assets<ColorMaterial>>,
                           mut viz_query: Query<(Entity, &TilePos, &mut TileColor), With<InVisionMarker>>,
                           mut noviz_query: Query<(Entity, &TilePos, &mut TileColor), Without<InVisionMarker>>) {
    for (e, tilepos, mut color) in viz_query.iter_mut() {
        let wt = st_world.world.map.get_tile_at(IVec2::new(tilepos.x as i32, tilepos.y as i32)).unwrap();
        *color = TileColor::from(Color::WHITE);
    }
    for (e, tilepos, mut color) in noviz_query.iter_mut() {
        let wt = st_world.world.map.get_tile_at(IVec2::new(tilepos.x as i32, tilepos.y as i32)).unwrap();
        *color = TileColor::from(Color::BLACK);
    }
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
        app.register_type::<MapTile>();
        app.insert_resource(StibagWorldRes {
            world,
        });
        app.insert_resource(StibagRes {
            BLACK: Handle::default(),
            rect_mesh: None,
        });
        app.add_event::<PlayerMovementEvent>();
        app.add_systems(Startup, plugin_init);
        app.add_systems(Update, gamepad_connections);
        app.add_systems(Update, gamepad_input_events);
        app.add_systems(Update, player_movement_sys);
        app.add_systems(Update, camera_recenter_sys);
        app.add_systems(Update, reassign_vision_markers_sys.after(player_movement_sys));
        app.add_systems(Update, set_material_colors_sys.after(reassign_vision_markers_sys));
    }
}
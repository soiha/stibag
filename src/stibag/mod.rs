pub mod map;
pub mod core;

use std::fs;
use std::sync::{Arc, Mutex};
use bevy::prelude::*;
use bevy::input::gamepad::GamepadConnection;
use bevy::input::gamepad::GamepadEvent;
use bevy::app::App;
use bevy::math::vec3;
use bevy_ecs_tilemap::prelude::*;
use bladeink;
use bladeink::story_error::StoryError;

const TILE_SIZE: f32 = 32.0;

#[derive(Resource)]
pub struct StibagWorldRes {
    story: Option<Arc<Mutex<bladeink::story::Story>>>,
    world: core::World,
}

impl FromWorld for StibagWorldRes {
    fn from_world(world: &mut World) -> Self {
        // let as = world.get_resource::<AssetServer>().unwrap().clone();
        let story_str = include_str!("../../assets/story.json");
        let story = bladeink::story::Story::new(story_str);
        if story.is_err() {
            error!("Failed to load story: {:?}", story.as_ref().err());
        } else {
            info!("Story loaded");
        }
        let mut st_world = core::World::init();
        let p_actor = st_world.spawn_actor_from_template("player".to_string());
        st_world.player_possess_actor(p_actor);
        StibagWorldRes {
            world: st_world,
            story: story.map_or(None, |s| Some(Arc::new(Mutex::new(s)))),
        }
    }
}

unsafe impl Send for StibagWorldRes {}

unsafe impl Sync for StibagWorldRes {}

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

fn plugin_init(mut commands: Commands, asset_server: Res<AssetServer>,
               st_world: Res<StibagWorldRes>,
               mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
               mut choice_ev: EventWriter<StoryChoiceEvent>,
               // array_texture_loader?
) {
    choice_ev.send(StoryChoiceEvent("Begin".to_string()));
    let tiles_tex_handle = asset_server.load("u5_tiles.png");
    let tex_layout = TextureAtlasLayout::from_grid(Vec2::new(32.0, 32.0), 32, 16, None, None);
    let sprite_layout = texture_atlas_layouts.add(tex_layout);

    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        }, CameraMarker {},
    ));


    let base_offset = vec3(-00.0, -0.0, 0.0);
    let mut player_trans = Transform::from_translation(base_offset + Vec3::new(-1600.0, -1600.0, 5.0));
    player_trans.scale = Vec3::new(1.0, 1.0, 1.0);
    let _plr = commands.spawn((SpriteSheetBundle {
        sprite: Default::default(),
        transform: player_trans,
        global_transform: Default::default(),
        texture: tiles_tex_handle.clone(),
        atlas: TextureAtlas {
            layout: sprite_layout,
            index: 10 * 32 + 14,
        },
        visibility: Default::default(),
        inherited_visibility: Default::default(),
        view_visibility: Default::default(),
    }, PlayerMarker {}));


    let tmap_size = TilemapSize { x: st_world.world.map.width, y: st_world.world.map.height };
    let mut tile_storage = TileStorage::empty(tmap_size);
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
            }, )).id();
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
        texture: TilemapTexture::Single(tiles_tex_handle),
        tile_size,
        transform: get_tilemap_center_transform(&tmap_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });

    info!("Stibag plugin init");
}

fn entity_sprite_position_sys(mut query: Query<(&mut Transform, ), With<PlayerMarker>>,
                              mut tilemap_q: ParamSet<(Query<(&Transform, &TilemapType, &TilemapGridSize, &TileStorage), (With<TileStorage>, Without<PlayerMarker>, Without<CameraMarker>)>, )>,
                              st_world: ResMut<StibagWorldRes>) {
    let tmq = tilemap_q.p0();
    let (map_transform, map_type, grid_size, _tilemap_storage) = tmq.single();

    // possessed actor
    let plr_a = st_world.world.player_interface.possessed_actor;
    let plr_pos = st_world.world.get_actor_pos(plr_a);

    for (mut transform, ) in query.iter_mut() {
        let tpos = TilePos { x: plr_pos.x as u32, y: plr_pos.y as u32 };
        let world_pos = tpos.center_in_world(grid_size, map_type).extend(5.0);
        transform.translation = map_transform.translation + world_pos;
    }
}

fn player_movement_sys(mut plr_set: ParamSet<(Query<&mut Transform, With<PlayerMarker>>, )>,
                       mut ev_movement: EventReader<PlayerMovementEvent>, mut st_world: ResMut<StibagWorldRes>, ) {
    let plr_a = st_world.world.player_interface.possessed_actor;
    let mut p = plr_set.p0();
    let plr_transform = p.single_mut();
    let mut pos = plr_transform.translation;
    for ev in ev_movement.read() {
        let moved = st_world.world.try_move_actor_by(plr_a, IVec2::new(ev.0.x, ev.0.y));
        if moved {
            info!("Player moved by {:?} now at {:?}", ev.0, st_world.world.get_actor_pos(plr_a));
        } else {
            info!("Player could not move by {:?}", ev.0);
        }
        info!("Player now at {:?}", st_world.world.get_actor_pos(plr_a));
        pos.x += ev.0.x as f32 * TILE_SIZE;
        pos.y += ev.0.y as f32 * TILE_SIZE;
        // plr_transform.translation = pos;
        // TODO geo check
    }
}

fn camera_recenter_sys(mut cam_set: ParamSet<(Query<&mut Transform, (With<CameraMarker>, Without<PlayerMarker>)>, )>,
                       mut tilemap_q: ParamSet<(Query<(&Transform, &TilemapType, &TilemapGridSize, &TileStorage), (With<TileStorage>, Without<PlayerMarker>, Without<CameraMarker>)>, )>,
                       st_world: ResMut<StibagWorldRes>) {
    let plr_a = st_world.world.player_interface.possessed_actor;
    let tmq = tilemap_q.p0();
    let (map_transform, map_type, grid_size, _tilemap_storage) = tmq.single();
    let mut c = cam_set.p0();
    let mut cam_trans = c.single_mut();
    let plr_map_pos = st_world.world.get_actor_pos(plr_a);
    let tpos = TilePos { x: plr_map_pos.x as u32, y: plr_map_pos.y as u32 };
    let t_center = tpos.center_in_world(&grid_size, map_type).extend(1.0);
    cam_trans.translation = map_transform.translation + t_center;
}

fn reassign_vision_markers_sys(mut commands: Commands, st_world: Res<StibagWorldRes>, mut current_viz_query: Query<(Entity, ), With<InVisionMarker>>,
                               map_tile_query: Query<(Entity, &TilePos, &mut TileColor)>,
) {
    let plr_map_pos = st_world.world.get_possessed_actor_pos();
    let plr_vision = st_world.world.map.calc_vision(plr_map_pos, 50.0);
    for (e, ) in current_viz_query.iter_mut() {
        commands.entity(e).remove::<InVisionMarker>();
    }
    for (e, tile_pos, _tile_color) in map_tile_query.iter() {
        if plr_vision.contains(&IVec2::new(tile_pos.x as i32, tile_pos.y as i32)) {
            commands.entity(e).insert(InVisionMarker);
        }
    }
}

fn set_material_colors_sys(mut _commands: Commands, st_world: Res<StibagWorldRes>,
                           mut viz_query: Query<(Entity, &TilePos, &mut TileColor), With<InVisionMarker>>,
                           mut noviz_query: Query<(Entity, &TilePos, &mut TileColor), Without<InVisionMarker>>) {
    for (_e, tilepos, mut color) in viz_query.iter_mut() {
        let wt = st_world.world.map.get_tile_at(IVec2::new(tilepos.x as i32, tilepos.y as i32)).unwrap();
        let lval = st_world.world.get_light_value_at(IVec2::new(tilepos.x as i32, tilepos.y as i32));
        let ambient = st_world.world.get_ambient_light_value();
        let final_color = if lval.1 > 0.0 {
            wt.get_color() * lval.1
        } else {
            ambient.0 * ambient.1
        };
        *color = TileColor::from(final_color);
    }
    for (_e, _tilepos, mut color) in noviz_query.iter_mut() {
        *color = TileColor::from(Color::BLACK);
    }
}

fn gamepad_input_events(mut _commands: Commands, stibag_gamepad: Option<Res<StibagGamepad>>, mut gamepad_evr: EventReader<GamepadEvent>, mut ev_movement: EventWriter<PlayerMovementEvent>) {
    if let Some(gamepad) = stibag_gamepad {
        for ev in gamepad_evr.read() {
            match ev {
                GamepadEvent::Button(input) => {
                    if input.gamepad.id == gamepad.0.id && input.value > 0.0 {
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

#[derive(Event)]
struct StoryTextEvent(String);

#[derive(Event)]
struct StoryChoiceEvent(String);  // something somewhere has caused a potential to select a (known) story choice by text

#[derive(Event)]
struct StoryChoiceEventWithIndex(usize); // something somewhere has caused a potential to select a story choice by index; mostly used for dialogues

#[derive(Event)]
// story output had some embedded tags. the tags_parser_sys will resolve these into events
struct StoryTagsEvent(String, String);

#[derive(Event)]
struct ChangeMapEvent(String); // change the map to the one specified

fn story_tag_handler_sys(mut commands: Commands, mut ev_tags: EventReader<StoryTagsEvent>, mut st_world: ResMut<StibagWorldRes>) {
    for ev in ev_tags.read() {
        let (tag, args) = (ev.0.clone(), ev.1.clone());
        match tag.as_str() {
            "change_map" => {
                unimplemented!();
            }
            _ => {
                error!("Unknown tag event: {:?}/{}", tag, args);
            }
        }
    }
}

fn story_progression_sys(mut commands: Commands, mut st_world: ResMut<StibagWorldRes>,
                         mut ev_story_text: EventWriter<StoryTextEvent>,
                         mut ev_tags: EventWriter<StoryTagsEvent>,
                         mut ev_story_choice: EventReader<StoryChoiceEvent>,
                         mut ev_story_choice_idx: EventReader<StoryChoiceEventWithIndex>, ) {
    let mut story = st_world.story.as_ref().unwrap().lock().unwrap();

    if story.can_continue() {
        let next = story.cont();
        let tags = story.get_current_tags();
        info!("Story continued");
        if let Some(tags_vec) = tags.as_ref().ok() {
            for tag in tags_vec.iter() {
                if tag.contains(":") {
                    let tag_parts: Vec<&str> = tag.split(":").collect();
                    match tag_parts.len() {
                        1 => {
                            let tag_ev = StoryTagsEvent(tag_parts[0].to_string(), "".to_string());
                            info!("Tag event with {}", tag_parts[0]);
                            ev_tags.send(tag_ev);
                        }
                        2 => {
                            let tag_ev = StoryTagsEvent(tag_parts[0].to_string(), tag_parts[1].to_string());
                            info!("Tag event with {}/{}", tag_parts[0], tag_parts[1]);
                            ev_tags.send(tag_ev);
                        }
                        _ => {
                            error!("Tag event with invalid parts: {:?}", tag_parts);
                        }
                    }
                }
            }
        }

        if let Some(ss) = next.as_ref().ok() {
            ev_story_text.send(StoryTextEvent(ss.to_string()));
            info!("Story continued: {:?}", ss);
        } else {
            error!("Failed to continue story: {:?}", next.as_ref().err());
        }
    } else {
        let choices = story.get_current_choices();
        for ev_ch in ev_story_choice.read() {
            let ch = choices.iter().find(|c| c.text == ev_ch.0);
            if let Some(choice) = ch {
                let res = story.choose_choice_index(choice.index.clone().into_inner());
                if res.is_err() {
                    error!("Failed to choose choice: {:?}", res.as_ref().err());
                } else {
                    info!("Chose choice: {:?}", choice.text);
                }
            } else {
                error!("Tried to select choice: {:?} but it is not available", ev_ch.0);
            }
        }

        for ev_ch in ev_story_choice_idx.read() {
            let ch = choices.get(ev_ch.0);
            if let Some(choice) = ch {
                let res = story.choose_choice_index(choice.index.clone().into_inner());
                if res.is_err() {
                    error!("Failed to choose choice: {:?}", res.as_ref().err());
                } else {
                    info!("Chose choice: {:?}", choice.text);
                }
            } else {
                error!("Tried to select choice with index {:?} but it is not available", ev_ch.0);
            }
        }
        /*
        info!("Current choices:", );
        for c in choices.iter() {
            info!("Choice: {:?}", c.text);
        }
         */
    }
}

impl Plugin for StibagGamePlugin {
    fn build(&self, app: &mut App) {
        let mut world = core::World::init();

        world.tick();

        app.init_resource::<StibagWorldRes>();


        app.add_event::<PlayerMovementEvent>();
        app.add_event::<StoryTextEvent>();
        app.add_event::<StoryChoiceEvent>();
        app.add_event::<StoryChoiceEventWithIndex>();
        app.add_event::<StoryTagsEvent>();

        app.add_systems(Startup, plugin_init);
        app.add_systems(Update, gamepad_connections);
        app.add_systems(Update, gamepad_input_events);
        app.add_systems(Update, player_movement_sys);
        app.add_systems(Update, entity_sprite_position_sys.after(player_movement_sys));
        app.add_systems(Update, camera_recenter_sys);
        app.add_systems(Update, reassign_vision_markers_sys.after(player_movement_sys));
        app.add_systems(Update, set_material_colors_sys.after(reassign_vision_markers_sys));
        app.add_systems(Update, story_progression_sys);
        app.add_systems(Update, story_tag_handler_sys.after(story_progression_sys));
    }
}
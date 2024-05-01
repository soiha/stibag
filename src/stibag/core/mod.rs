use bevy::render::color::Color;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use bevy::log::info;
use bevy::math::{IVec2, Vec4};
use koto::Koto;
use crate::stibag;
use crate::stibag::map::{LightContribution, LightEmitter};

pub type ItemId = u32;
pub type ActorId = u32;

pub type LightId = u32;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum ItemSlot {
    None,
    Head,
    Neck,
    Shoulders,
    Chest,
    Back,
    Wrists,
    Hands,
    Waist,
    Legs,
    Feet,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
    MainHand,
    OffHand,
}

#[allow(dead_code)]
pub trait Item {
    fn id(&self) -> ItemId;

    fn set_parent_container(&mut self, container_id: ItemId);
    fn parent_container(&self) -> ItemId;
    fn slot(&self) -> ItemSlot;
    fn display_name_singular(&self) -> String;
    fn weight(&self) -> f32;

    fn container(&self) -> Option<&ItemContainer>;
}

#[allow(dead_code)]
pub trait WorldActor {
    fn actor_id(&self) -> ActorId;

    fn info(&mut self) -> &mut ActorInfo;

    fn position(&mut self) -> IVec2 {
        self.info().position.clone()
    }

    fn on_spawn(&self, world: &mut World);
    fn act(&self, world: &mut World) -> u64;

    fn move_to(&mut self, new_position: IVec2) {
        self.info().position = new_position;
    }
    fn on_move(&mut self, world: &mut World, new_position: IVec2);
}

struct BasicItem {
    id: ItemId,
    parent_container: ItemId,
    slot: ItemSlot,
    display_name: String,
    weight: f32,
}

impl Item for BasicItem {
    fn id(&self) -> ItemId {
        self.id
    }

    fn set_parent_container(&mut self, container_id: ItemId) {
        self.parent_container = container_id;
    }

    fn parent_container(&self) -> ItemId {
        self.parent_container
    }

    fn slot(&self) -> ItemSlot {
        self.slot
    }

    fn display_name_singular(&self) -> String {
        self.display_name.clone()
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn container(&self) -> Option<&ItemContainer> {
        None
    }
}

pub struct ActorInfo {
    pub position: IVec2,
}

impl ActorInfo {
    pub fn new() -> Self {
        ActorInfo {
            position: IVec2::new(0, 0),
        }
    }
}

#[allow(dead_code)]
struct HumanoidActor {
    pub actor_id: ActorId,
    pub info: ActorInfo,
    pub inventory: ItemContainer,
    pub vision_radius: f32,
    pub vision: Vec<IVec2>,
}

pub struct PlayerInterface {
    pub possessed_actor: ActorId,
}


impl WorldActor for HumanoidActor {
    fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    fn info(&mut self) -> &mut ActorInfo {
        &mut self.info
    }

    fn on_spawn(&self, _world: &mut World) {
        info!("Humanoid actor {} spawned", self.actor_id);
    }

    fn on_move(&mut self, world: &mut World, new_position: IVec2) {
        info!("Humanoid actor {} moved to {:?}", self.actor_id, new_position);
        self.vision = world.map.calc_vision(new_position, self.vision_radius);
    }

    fn act(&self, _world: &mut World) -> u64 {
        info!("Humanoid actor {} acting", self.actor_id);
        1
    }
}

#[allow(dead_code)]
pub struct ItemContainer {
    id: ItemId,
    contents: Vec<Box<dyn Item + Send + Sync>>,
}

#[allow(dead_code)]
impl ItemContainer {
    pub fn new() -> Self {
        ItemContainer {
            id: 0,
            contents: Vec::new(),
        }
    }

    pub fn can_contain(&self, _item: &Box<dyn Item + Send + Sync>) -> bool {
        true
    }

    pub fn add_item(&mut self, mut item: Box<dyn Item + Send + Sync>) {
        if self.can_contain(&item) {
            item.set_parent_container(self.id);
            self.contents.push(item);
        }
    }

    pub fn remove_item(&mut self, item_id: ItemId) {
        self.contents.retain(|item| item.id() != item_id);
    }

    pub fn get_item(&self, item_id: ItemId) -> Option<&Box<dyn Item + Send + Sync>> {
        self.contents.iter().find(|item| item.id() == item_id)
    }
}

#[allow(dead_code)]
pub struct World {
    pub koto_env: Koto,
    pub player_interface: PlayerInterface,
    pub map: stibag::map::Map,
    pub current_timeslice: u64,
    actor_id_count: u64,
    item_id_count: u64,
    light_id_count: u64,
    pub timeline: Arc<Mutex<Vec<(u64, ActorId)>>>,
    pub actors: Arc<Mutex<HashMap<ActorId, Box<dyn WorldActor + Send + Sync>>>>,
    pub items: Arc<Mutex<HashMap<ItemId, Box<dyn Item + Send + Sync>>>>,
    pub lights: Arc<Mutex<HashMap<LightId, Box<LightEmitter>>>>,
}

#[allow(dead_code)]
impl World {
    pub fn init() -> Self {
        let mut w = World {
            player_interface: PlayerInterface {
                possessed_actor: 0,
            },
            koto_env: Koto::default(),
            map: stibag::map::Map::new_from_template("default".to_string(), bevy::math::IVec2::new(100, 100)),
            current_timeslice: 0,
            actor_id_count: 1,
            item_id_count: 1,
            light_id_count: 1,
            timeline: Arc::new(Mutex::new(Vec::new())),
            actors: Arc::new(Mutex::new(HashMap::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
            lights: Arc::new(Mutex::new(HashMap::new())),
        };
        w.map.blit_tiles_from_charmap(IVec2::new(5, 5), vec![
            "########".into(),
            "#......#".into(),
            ".......#".into(),
            "#......#".into(),
            "########".into(),
        ], |c| match c {
            '#' => Some(stibag::map::MapTile {
                tile_type: "wall".to_string(),
                tile_visual: "wall".to_string(),
                position: bevy::math::IVec2::new(0, 0),
                contained_items: ItemContainer::new(),
                transparency: stibag::map::Transparency::Opaque,
                light_color: Color::BLACK,
                light_amount: 0.0,
                traversal_cost: -1.0,
                lighting: Vec::new(),
            }),
            _ => None
        });
        w.spawn_light(IVec2::new(5, 4), None, Color::WHITE, 1.0);
        w.spawn_light(IVec2::new(10, 1), None, Color::AQUAMARINE, 1.0);
        w.recalculate_lighting();
        info!("World initialized!");
        w
    }

    pub fn spawn_light(&mut self, position: IVec2, parent_actor: Option<ActorId>, color: Color, initial_intensity: f32) -> LightId {
        let light_id = self.light_id_count;
        self.light_id_count += 1;
        let new_emitter = Box::new(LightEmitter {
            light_id: light_id.try_into().unwrap(),
            position,
            parent_actor,
            color: color.clone(),
            intensity: initial_intensity,
        });
        let l_cloned = self.lights.clone();
        let mut map = l_cloned.lock().unwrap();
        map.insert(light_id.try_into().unwrap(), new_emitter);
        light_id.try_into().unwrap()
    }

    pub fn spawn_actor_from_template(&mut self, _template: String) -> ActorId {
        let actor_id = self.actor_id_count;
        self.actor_id_count += 1;
        let newactor = Box::new(HumanoidActor {
            actor_id: actor_id.try_into().unwrap(),
            info: ActorInfo::new(),
            vision: Vec::new(),
            vision_radius: 30.0,
            inventory: ItemContainer::new(),
        });
        newactor.on_spawn(self);
        let ac = self.actors.clone();
        let mut map = ac.lock().unwrap();
        map.insert(actor_id.try_into().unwrap(), newactor);
        self.place_on_timeline(actor_id.try_into().unwrap(), self.current_timeslice + 1);
        actor_id.try_into().unwrap()
    }

    pub fn spawn_item_from_template(&mut self, _template: String) -> ItemId {
        let item_id = self.item_id_count;
        self.item_id_count += 1;
        let newitem = Box::new(BasicItem {
            id: item_id.try_into().unwrap(),
            parent_container: 0,
            slot: ItemSlot::None,
            display_name: "A basic item".to_string(),
            weight: 0.0,
        });
        let i_cloned = self.items.clone();
        let mut map = i_cloned.lock().unwrap();
        map.insert(item_id.try_into().unwrap(), newitem);
        item_id.try_into().unwrap()
    }

    pub fn set_action_timeslice_on_timeline_for(&mut self, actor_id: ActorId, target_timeslice: u64) {
        info!("Setting action timeslice for actor {} to {}", actor_id, target_timeslice);
        let tl_clone = self.timeline.clone();
        let mut tl = tl_clone.lock().unwrap();
        tl.retain(|(_ts, aid)| aid != &actor_id);
        drop(tl);
        self.place_on_timeline(actor_id, target_timeslice);
        info!(" => done");
    }
    pub fn is_on_timeline(&self, actor_id: ActorId) -> bool {
        let cloned = self.timeline.clone();
        let tl = cloned.lock().unwrap();
        tl.iter().any(|(_ts, aid)| aid == &actor_id)
    }
    pub fn place_on_timeline(&mut self, actor_id: ActorId, timeslice: u64) {
        info!("Placing actor {} on timeline at {}", actor_id, timeslice);
        let on_timeline = self.is_on_timeline(actor_id);
        let tl_clone = self.timeline.clone();
        let mut tl = tl_clone.lock().unwrap();

        if on_timeline {
            tl.retain(|(_ts, aid)| aid != &actor_id);
        }
        tl.push((timeslice, actor_id));
        tl.sort_by(|a, b| a.0.cmp(&b.0));
        info!(" => done");
    }
    pub fn player_possess_actor(&mut self, actor_id: ActorId) {
        self.player_interface.possessed_actor = actor_id;
        info!("Player possessed actor {}", actor_id);
    }

    pub fn get_possessed_actor_pos(&self) -> IVec2 {
        self.get_actor_pos(self.player_interface.possessed_actor)
    }

    pub fn get_actor_pos(&self, actor_id: ActorId) -> IVec2 {
        let ac = self.actors.clone();
        let mut map = ac.lock().unwrap();
        let actor = map.get_mut(&actor_id).unwrap();
        let ret = actor.position().clone();
        drop(map);
        ret
    }
    pub fn try_move_actor_to(&mut self, actor_id: ActorId, new_position: IVec2) -> bool {
        let ac = self.actors.clone();
        let mut map = ac.lock().unwrap();
        let actor = map.get_mut(&actor_id).unwrap();
        let tile = self.map.get_tile_at(new_position);
        if let Some(t) = tile {
            if t.traversal_cost > 0.0 {
                actor.move_to(new_position);
                actor.on_move(self, new_position);
                drop(map);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn try_move_actor_by(&mut self, actor_id: ActorId, delta: IVec2) -> bool {
        let ac = self.actors.clone();
        let mut map = ac.lock().unwrap();
        let actor = map.get_mut(&actor_id).unwrap();
        let apos = actor.position().clone();
        drop(map);
        let new_position = apos + delta;
        self.try_move_actor_to(actor_id, new_position)
    }

    pub fn get_ambient_light_value(&self) -> (Color, f32) {
        (Color::WHITE, 0.25)
    }
    pub fn get_light_value_at(&self, position: IVec2) -> (Color, f32) {
        self.map.get_tile_at(position).map(|tile| (tile.light_color, tile.light_amount)).unwrap_or((Color::BLACK, 0.0))
    }
    pub fn recalculate_lighting(&mut self) {
        let l_cloned = self.lights.clone();
        let mut l = l_cloned.lock().unwrap();

        let amb = self.get_ambient_light_value();
        self.map.tiles.iter_mut().for_each(|tile| {
            tile.lighting.clear();
            let ambient = LightContribution::new_ambient(amb.0, amb.1);
            tile.lighting.push(ambient);
        });

        for (_lid, emitter) in l.iter_mut() {
            let pos = emitter.position;
            let l_color = emitter.color;
            let intensity = emitter.intensity;
            let light_vision = self.map.calc_vision(pos, 30.0);
            for pos in light_vision {
                let tile = self.map.get_tile_at_mut(pos);

                let dist = (emitter.position.distance_squared(pos) as f32).sqrt();
                info!("RELIGHT {} <=> {} dist={}", emitter.position, pos, dist);
                let l_intensity = if dist > 0.0 {
                    intensity / dist
                } else {
                    emitter.intensity
                };

                let color = Color::rgba(
                    l_color.r() * l_intensity,
                    l_color.g() * l_intensity,
                    l_color.b() * l_intensity,
                    1.0);


                tile.light_color = tile.light_color.add(color);
                let mut tcv: Vec4 = tile.light_color.rgba_to_vec4();
                tcv = tcv.normalize();
                tile.light_color = Color::rgba(tcv.x, tcv.y, tcv.z, tcv.w);
                tile.light_amount += l_intensity;
                tile.lighting.push(LightContribution::new_emitter(emitter.light_id, color, l_intensity));
            }
        }
    }

    pub fn tick(&mut self) -> bool {
        self.current_timeslice += 1;
        let tl_clone = self.timeline.clone();
        let tl = tl_clone.lock().unwrap();
        let next = tl.get(0);

        let mut ret = false;
        if let Some((ts, aid)) = next.clone() {
            if ts == &self.current_timeslice {
                let ac = self.actors.clone();
                let mut map = ac.lock().unwrap();
                let actor = map.get_mut(aid).unwrap();
                let delay = actor.act(self);
                let a_id = *aid;
                let target = self.current_timeslice + delay;
                drop(tl);
                drop(tl_clone);
                self.set_action_timeslice_on_timeline_for(a_id, target);
                ret = true;
            }
        }
        ret
    }

    pub fn tick_until(&mut self, target_timeslice: u64) {
        while self.current_timeslice < target_timeslice {
            self.tick();
        }
    }
}
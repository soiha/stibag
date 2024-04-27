use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bevy::log::info;
use koto::Koto;
use crate::stibag;

type ItemId = u32;
type ActorId = u32;

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

pub trait Item {
    fn id(&self) -> ItemId;
    
    fn set_parent_container(&mut self, container_id: ItemId);
    fn parent_container(&self) -> ItemId;
    fn slot(&self) -> ItemSlot;
    fn display_name_singular(&self) -> String;
    fn weight(&self) -> f32;
    
    fn container(&self) -> Option<&ItemContainer>;
}

trait WorldActor {
    fn actor_id(&self) -> ActorId;
    
    fn on_spawn(&self, world: &mut World);
    fn act(&self, world: &mut World) -> u64;
    
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

struct HumanoidActor {
    pub actor_id: ActorId,
    pub inventory: ItemContainer,
    
}

struct PlayerInterface {
    possessed_actor: ActorId,
}

impl WorldActor for HumanoidActor {
    fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    fn on_spawn(&self, world: &mut World) {
        info!("Humanoid actor {} spawned", self.actor_id);
    }

    fn act(&self, world: &mut World) -> u64 {
        info!("Humanoid actor {} acting", self.actor_id);
        1
    }
}


pub struct ItemContainer {
    id: ItemId,
    contents: Vec<Box<dyn Item>>,
}

impl ItemContainer {
    pub fn new() -> Self {
        ItemContainer {
            id: 0,
            contents: Vec::new(),
        }
    }
    
    pub fn can_contain(&self, item: &Box<dyn Item>) -> bool {
        true
    }
    
    pub fn add_item(&mut self, mut item: Box<dyn Item>) {
        if self.can_contain(&item) {
            item.set_parent_container(self.id);
            self.contents.push(item);
        }   
    }
    
    pub fn remove_item(&mut self, item_id: ItemId) {
        self.contents.retain(|item| item.id() != item_id);
    }
    
    pub fn get_item(&self, item_id: ItemId) -> Option<&Box<dyn Item>> {
        self.contents.iter().find(|item| item.id() == item_id)
    }
}

pub struct World {
    koto_env: Koto,
    player_interface: PlayerInterface,
    map: stibag::map::Map,
    current_timeslice: u64,
    actor_id_count: u64,
    item_id_count: u64,
    timeline: Arc<Mutex<Vec<(u64, ActorId)>>>,
    actors: Arc<Mutex<HashMap<ActorId, Box<dyn WorldActor>>>>,
    items: Arc<Mutex<HashMap<ItemId, Box<dyn Item>>>>,
}

impl World {
    pub fn init() -> Self {
        let w = World {
            koto_env: Koto::default(),
            player_interface: PlayerInterface {
                possessed_actor: 0,
            },
            map: stibag::map::Map::new_from_template("default".to_string(), bevy::math::IVec2::new(100, 100)),
            current_timeslice: 0,
            actor_id_count: 1,
            item_id_count: 1,
            timeline: Arc::new(Mutex::new(Vec::new())),
            actors: Arc::new(Mutex::new(HashMap::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
        };
        info!("World initialized!");
        w
    }

    pub fn spawn_actor_from_template(&mut self, template: String) -> ActorId {
        let actor_id = self.actor_id_count;
        self.actor_id_count += 1;
        let newactor = Box::new(HumanoidActor {
            actor_id: actor_id.try_into().unwrap(),
            inventory: ItemContainer::new(),
        });
        newactor.on_spawn(self);
        let ac = self.actors.clone();
        let mut map = ac.lock().unwrap();
        map.insert(actor_id.try_into().unwrap(), newactor);
        self.place_on_timeline(actor_id.try_into().unwrap(), self.current_timeslice + 1);
        actor_id.try_into().unwrap()
    }
    
    pub fn spawn_item_from_template(&mut self, template: String) -> ItemId {
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
        let mut tl_clone = self.timeline.clone();
        let mut tl = tl_clone.lock().unwrap();
        tl.retain(|(ts, aid)| aid != &actor_id);
        drop(tl);
        self.place_on_timeline(actor_id, target_timeslice);
        info!(" => done");
    }
    pub fn is_on_timeline(&self, actor_id: ActorId) -> bool {
        let mut cloned = self.timeline.clone();
        let mut tl = cloned.lock().unwrap();
        tl.iter().any(|(ts, aid)| aid == &actor_id)
    }
    pub fn place_on_timeline(&mut self, actor_id: ActorId, timeslice: u64) {
        info!("Placing actor {} on timeline at {}", actor_id, timeslice);
        let on_timeline = self.is_on_timeline(actor_id);
        let mut tl_clone = self.timeline.clone();
        let mut tl = tl_clone.lock().unwrap();
        
        if on_timeline {
            tl.retain(|(ts, aid)| aid != &actor_id);
        }
        tl.push((timeslice, actor_id));
        tl.sort_by(|a, b| a.0.cmp(&b.0));
        info!(" => done");
    }
    pub fn player_possess_actor(&mut self, actor_id: ActorId) {
        self.player_interface.possessed_actor = actor_id;
        info!("Player possessed actor {}", actor_id);
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
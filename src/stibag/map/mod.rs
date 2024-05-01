use crate::stibag::core::{ActorId, LightId};
use bevy::math::IVec2;
use bevy::prelude::Color;
use bevy_ecs_tilemap::prelude::TileTextureIndex;
use crate::stibag::core::ItemContainer;

type TileTypeId = String;
type TileVisualId = String;


#[allow(dead_code)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WrapMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum Transparency {
    #[default]
    Opaque,
    Transparent,
}

#[derive(Debug, Default)]
pub enum LightContributionType {
    #[default]
    Ambient,
    Emitter(LightId),
}

#[derive(Debug, Default)]
pub struct LightContribution {
    pub light_contribution_type: LightContributionType,
    pub color: Color,
    pub intensity: f32,
}

impl LightContribution {
    pub fn new_ambient(color: Color, intensity: f32) -> Self {
        LightContribution {
            light_contribution_type: LightContributionType::Ambient,
            color,
            intensity,
        }
    }

    pub fn new_emitter(light_id: LightId, color: Color, intensity: f32) -> Self {
        LightContribution {
            light_contribution_type: LightContributionType::Emitter(light_id),
            color,
            intensity,
        }
    }
}

pub struct LightEmitter {
    pub light_id: LightId,
    pub parent_actor: Option<ActorId>,
    pub position: IVec2,
    pub color: Color,
    pub intensity: f32,
}

pub struct MapTile {
    pub tile_type: TileTypeId,
    pub tile_visual: TileVisualId,
    pub light_color: Color,
    pub light_amount: f32,
    pub position: IVec2,
    pub contained_items: ItemContainer,
    pub transparency: Transparency,
    pub traversal_cost: f32,
    pub lighting: Vec<LightContribution>,
}


impl MapTile {
    pub fn clone_without_inventory(&self) -> Self {
        MapTile {
            tile_type: "grass".to_string(),
            tile_visual: "grass".to_string(),
            position: IVec2::new(0, 0),
            contained_items: ItemContainer::new(),
            transparency: self.transparency,
            traversal_cost: self.traversal_cost,
            light_color: self.light_color.clone(), // the combined color of lights that have contributed to this tile
            light_amount: self.light_amount,
            lighting: Vec::new(), // all light contributions to this tile
        }
    }

    pub fn get_texture_index(&self) -> TileTextureIndex {
        match self.tile_visual.as_str() {
            "grass" => TileTextureIndex(5),
            "wall" => TileTextureIndex(32 * 2 + 15),
            "water" => TileTextureIndex(3),
            "sand" => TileTextureIndex(7),
            _ => TileTextureIndex(8 * 32 + 32),
        }
    }

    pub fn get_color(&self) -> bevy::render::color::Color {
        if self.position.x == 0 && self.position.y == 0 {
            return bevy::render::color::Color::rgb(1.0, 0.0, 0.0);
        }
        match self.tile_visual.as_str() {
            "grass" => bevy::render::color::Color::rgb(0.0, 1.0, 0.0),
            "wall" => bevy::render::color::Color::rgb(0.5, 0.5, 0.5),
            "water" => bevy::render::color::Color::rgb(1.0, 0.0, 1.0),
            "sand" => bevy::render::color::Color::rgb(1.0, 1.0, 0.0),
            _ => bevy::render::color::Color::rgb(1.0, 1.0, 1.0),
        }
    }
}

trait FOVQuery {
    fn is_blocked(&self, x: i32, y: i32) -> bool;
    fn radius(&self, x: f32, y: f32) -> f32;
}

impl FOVQuery for Map {
    fn is_blocked(&self, x: i32, y: i32) -> bool {
        let tile = self.get_tile_at(IVec2::new(x, y)).unwrap();
        tile.transparency == Transparency::Opaque
    }

    fn radius(&self, x: f32, y: f32) -> f32 {
        (x * x + y * y).sqrt()
    }
}

struct FOVCalc<'a> {
    pub startx: i32,
    pub starty: i32,
    pub width: usize,
    pub height: usize,
    pub radius: f32,
    pub map_query: &'a dyn FOVQuery,
    pub results: Vec<IVec2>,
}

impl<'a> FOVCalc<'a> {
    pub fn start_new(startx: i32, starty: i32, radius: f32, map_width: usize, map_height: usize,
                     map_query: &'a dyn FOVQuery) -> Self {
        FOVCalc {
            startx,
            starty,
            width: map_width,
            height: map_height,
            radius,
            map_query,
            results: Vec::new(),
        }
    }

    pub fn calculate(&mut self) {
        self.results = Vec::new();
        let diagonals: [IVec2; 4] = [
            IVec2::new(-1, -1),
            IVec2::new(-1, 1),
            IVec2::new(1, -1),
            IVec2::new(1, 1),
        ];
        self.results.push(IVec2::new(self.startx, self.starty));
        for d in diagonals {
            self.castlight(1, 1.0, 0.0, 0, d.x, d.y, 0);
            self.castlight(1, 1.0, 0.0, d.x, 0, 0, d.y);
        }
    }
    fn castlight(&mut self, row: i32, mut start: f32, end: f32, xx: i32, xy: i32, yx: i32, yy: i32) {
        let radius = 30;

        let mut newstart: f32 = 0.0;
        if start < end {
            return;
        }
        let mut blocked = false;
        let mut distance = row;
        while distance <= radius && !blocked {
            let delta_y = -distance;
            for delta_x in -distance..=0 {
                let current_x = self.startx + xx * delta_x + xy * delta_y;
                let current_y = self.starty + yx * delta_x + yy * delta_y;
                let left_slope = (delta_x as f32 - 0.5) / (delta_y as f32 + 0.5);
                let right_slope = (delta_x as f32 + 0.5) / (delta_y as f32 - 0.5);

                if !(current_x >= 0 && current_y >= 0 && current_x < self.width as i32 && current_y < self.height as i32) || start < right_slope {
                    continue;
                } else if end > left_slope {
                    break;
                }

                if self.map_query.radius(delta_x as f32, delta_y as f32) <= self.radius {
                    let pos = IVec2::new(current_x, current_y);
                    self.results.push(pos);
                }

                if blocked {
                    if self.map_query.is_blocked(current_x, current_y) {
                        newstart = right_slope;
                        continue;
                    } else {
                        blocked = false;
                        start = newstart;
                    }
                } else {
                    if self.map_query.is_blocked(current_x, current_y) && distance < self.radius as i32 {
                        blocked = true;
                        self.castlight(row + 1, start, left_slope, xx, xy, yx, yy);
                        newstart = right_slope;
                    }
                }
            }
            distance += 1;
        }
    }
}

pub struct Map {
    pub tiles: Vec<MapTile>,
    pub width: u32,
    pub height: u32,
    pub horizontal_wrap: WrapMode,
    pub vertical_wrap: WrapMode,
}

impl Map {
    pub fn new_from_template(_template: impl Into<String>, dimensions: IVec2) -> Self {
        let new_t = MapTile {
            tile_type: "grass".to_string(),
            tile_visual: "grass".to_string(),
            position: IVec2::new(0, 0),
            contained_items: ItemContainer::new(),
            transparency: Transparency::Transparent,
            light_amount: 0.0,
            light_color: Color::BLACK,
            traversal_cost: 1.0,
            lighting: Vec::new(),
        };
        let mut tiles = Vec::with_capacity((dimensions.x * dimensions.y) as usize);
        for y in 0..dimensions.y {
            for x in 0..dimensions.x {
                let mut push_t = new_t.clone_without_inventory();
                push_t.position = IVec2::new(x, y);
                tiles.push(push_t);
            }
        }
        Map {
            tiles,
            width: dimensions.x as u32,
            height: dimensions.y as u32,
            horizontal_wrap: WrapMode::Repeat,
            vertical_wrap: WrapMode::Repeat,
        }
    }

    pub fn blit_tile_at(&mut self, position: IVec2, mut tile: MapTile) {
        assert!(position.x < self.width as i32);
        assert!(position.y < self.height as i32);
        tile.position = position;

        let t: &mut MapTile = self.get_tile_at_mut(position);
        assert_eq!(t.position, position, "Tile position mismatch: wanted {} got {}", position, t.position);
        t.tile_type = tile.tile_type.clone();
        t.tile_visual = tile.tile_visual.clone();
        t.transparency = tile.transparency;
        t.traversal_cost = tile.traversal_cost;
    }

    pub fn get_tile_at_mut(&mut self, position: IVec2) -> &mut MapTile {
        let t = &mut self.tiles[(position.y * (self.width as i32) + position.x) as usize];
        t
    }

    pub fn get_tile_at(&self, position: IVec2) -> Option<&MapTile> {
        if position.x > self.width as i32 - 1 {
            match self.horizontal_wrap {
                WrapMode::Clamp => return self.get_tile_at(IVec2::new(self.width as i32 - 1, position.y)),
                WrapMode::Repeat => return self.get_tile_at(IVec2::new(position.x % self.width as i32, position.y)),
                WrapMode::Mirror => return self.get_tile_at(IVec2::new(self.width as i32 - position.x % self.width as i32, position.y)),
            }
        }
        if position.x < 0 {
            match self.horizontal_wrap {
                WrapMode::Clamp => return self.get_tile_at(IVec2::new(0, position.y)),
                WrapMode::Repeat => return self.get_tile_at(IVec2::new(self.width as i32 + position.x % self.width as i32, position.y)),
                WrapMode::Mirror => return self.get_tile_at(IVec2::new(-position.x % self.width as i32, position.y)),
            }
        }
        if position.y > self.height as i32 - 1 {
            match self.vertical_wrap {
                WrapMode::Clamp => return self.get_tile_at(IVec2::new(position.x, self.height as i32 - 1)),
                WrapMode::Repeat => return self.get_tile_at(IVec2::new(position.x, position.y % self.height as i32)),
                WrapMode::Mirror => return self.get_tile_at(IVec2::new(position.x, self.height as i32 - position.y % self.height as i32)),
            }
        }
        if position.y < 0 {
            match self.vertical_wrap {
                WrapMode::Clamp => return self.get_tile_at(IVec2::new(position.x, 0)),
                WrapMode::Repeat => return self.get_tile_at(IVec2::new(position.x, self.height as i32 + position.y % self.height as i32)),
                WrapMode::Mirror => return self.get_tile_at(IVec2::new(position.x, -position.y % self.height as i32)),
            }
        }
        let t = &self.tiles[(position.y * (self.width as i32) + position.x) as usize];
        assert!(t.position == position, "Tile position mismatch: wanted {} got {}", position, t.position);
        Some(t)
    }

    #[allow(dead_code)]
    #[allow(non_snake_case)]
    pub fn line_trace(&self, from: IVec2, to: IVec2, filter_func: fn(&Self, IVec2, &MapTile) -> bool) -> Vec<IVec2> {
        let mut result = Vec::new();
        let mut y = from.y;
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        let mut D = 2 * (dy - dx);
        for x in from.x..=to.x {
            if filter_func(self, IVec2::new(x, y), self.get_tile_at(IVec2::new(x, y)).unwrap()) {
                result.push(IVec2::new(x, y));
                if D > 0 {
                    y += 1;
                    D -= 2 * dx;
                }
                D += 2 * dy;
            }
        }
        result
    }

    #[allow(dead_code)]
    pub fn can_see(&self, from: IVec2, to: IVec2) -> bool {
        let line = self.line_trace(from, to, |_world, _pos, tile| {
            if tile.transparency == Transparency::Opaque {
                return false;
            }
            true
        });
        match line.last() {
            Some(last) => {
                last == &to
            }
            None => {
                false
            }
        }
    }

    pub fn blit_tiles_from_charmap(&mut self, top_left_pos: IVec2, charmap: Vec<String>, char_mapper_func: fn(char) -> Option<MapTile>) {
        for (y, row) in charmap.iter().enumerate() {
            println!("{}", row);
            for (x, c) in row.chars().enumerate() {
                if let Some(tile) = char_mapper_func(c) {
                    self.blit_tile_at(IVec2::new(top_left_pos.x + x as i32, top_left_pos.y + y as i32), tile);
                }
            }
        }
    }


    pub fn calc_vision(&self, from: IVec2, vis_radius: f32) -> Vec<IVec2> {
        let mut fov = FOVCalc::start_new(from.x, from.y, vis_radius, self.width as usize, self.height as usize, self);
        fov.calculate();
        fov.results
    }
}
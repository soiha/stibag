use bevy::math::IVec2;
use crate::stibag::core::ItemContainer;

type TileTypeId = String;
type TileVisualId = String;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WrapMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Transparency {
    Opaque,
    Transparent,
}


pub struct MapTile {
    pub tile_type: TileTypeId,
    pub tile_visual: TileVisualId,
    pub position: IVec2,
    pub contained_items: ItemContainer,
    pub transparency: Transparency,
    pub traversal_cost: f32,
}

impl MapTile {
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

pub struct Map {
    pub tiles: Vec<MapTile>,
    pub width: u32,
    pub height: u32,
    pub horizontal_wrap: WrapMode,
    pub vertical_wrap: WrapMode,
}

impl Map {
    pub fn new_from_template(template: impl Into<String>, dimensions: IVec2) -> Self {
        let mut tiles = Vec::new();
        for x in 0..dimensions.x {
            for y in 0..dimensions.y {
                tiles.push(MapTile {
                    tile_type: "grass".to_string(),
                    tile_visual: "grass".to_string(),
                    position: IVec2::new(x, y),
                    contained_items: ItemContainer::new(),
                    transparency: Transparency::Transparent,
                    traversal_cost: 1.0,
                });
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
        
        let mut t: &mut MapTile = self.get_tile_at_mut(position);
        t.tile_type = tile.tile_type.clone();
        t.tile_visual = tile.tile_visual.clone();
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
        let  t= &self.tiles[(position.y * (self.width as i32) + position.x) as usize];
        Some(t)
    }
    
    #[allow(non_snake_case)]
    pub fn line_trace(&self, from: IVec2, to: IVec2, filter_func: fn(IVec2, &MapTile) -> bool) -> Vec<IVec2> {
        let mut result = Vec::new();
        let mut x = from.x;
        let mut y = from.y;
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        let mut D = 2 * (dy - dx);
        for x in from.x..to.x {
            if filter_func(IVec2::new(x, y), self.get_tile_at(IVec2::new(x, y)).unwrap()) {
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
    
    pub fn can_see(&self, from: IVec2, to: IVec2) -> bool {
        let mut result = true;
        let line = self.line_trace(from, to, |pos, tile| {
            if tile.transparency == Transparency::Opaque {
                return false;
            }
            true
        });
        match line.last() {
            Some(last) => {
                last == &to
            },
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
}
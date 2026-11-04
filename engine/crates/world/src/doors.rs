use glam::IVec3;

use crate::block::{
    Block, DOOR_OPEN, DOOR_PLACED_RIGHT, GATE_FACING_MASK, GATE_OPEN, Shape, TRAPDOOR_OPEN,
    facing_code,
};
use crate::column::WORLD_HEIGHT;
use crate::world::World;

impl World {
    pub fn can_place_door(&self, pos: IVec3, lower: Block) -> bool {
        lower.door_upper().is_some()
            && pos.y + 1 < WORLD_HEIGHT
            && self.is_loaded(pos)
            && self.block(pos).is_replaceable()
            && self.block(pos + IVec3::Y).is_replaceable()
            && lower.stands_on(self.block(pos + IVec3::NEG_Y))
    }

    /// Петля новой двери, как `ItemDoor.placeDoor`: соседние двери, стены и точка клика.
    pub fn door_hinge_right(
        &self,
        pos: IVec3,
        lower: Block,
        facing: IVec3,
        hit: (f32, f32),
    ) -> bool {
        let right = IVec3::new(-facing.z, 0, facing.x);
        let (at_left, at_right) = (pos - right, pos + right);
        let wall = |at: IVec3| {
            let solid = |p: IVec3| {
                let block = self.block(p);
                i32::from(block.is_opaque() && block.shape() == Shape::Cube)
            };
            solid(at) + solid(at + IVec3::Y)
        };
        let walls = wall(at_right) - wall(at_left);
        let door = |at: IVec3| {
            lower.same_door(self.block(at)) || lower.same_door(self.block(at + IVec3::Y))
        };
        let (door_left, door_right) = (door(at_left), door(at_right));
        if (door_left && !door_right) || walls > 0 {
            return true;
        }
        if (door_right && !door_left) || walls < 0 {
            return false;
        }
        let (hit_x, hit_z) = hit;
        (facing.x < 0 && hit_z < 0.5)
            || (facing.x > 0 && hit_z > 0.5)
            || (facing.z < 0 && hit_x > 0.5)
            || (facing.z > 0 && hit_x < 0.5)
    }

    /// `data` — как у нижней половины плюс `DOOR_PLACED_RIGHT`, если петля справа.
    pub fn place_door(&mut self, pos: IVec3, lower: Block, data: u8) -> Vec<IVec3> {
        let Some(upper) = lower.door_upper() else {
            return Vec::new();
        };
        let facing = data & GATE_FACING_MASK;
        let hinge = u8::from(data & DOOR_PLACED_RIGHT != 0);
        if !self.set_block_with_data(pos, lower, facing)
            || !self.set_block_with_data(pos + IVec3::Y, upper, hinge)
        {
            return Vec::new();
        }
        vec![pos, pos + IVec3::Y]
    }

    /// Правая кнопка по калитке или деревянной двери; `look` — взгляд игрока.
    pub fn toggle_by_hand(&mut self, pos: IVec3, look: IVec3) -> Vec<IVec3> {
        let block = self.block(pos);
        if !block.opens_by_hand() {
            return Vec::new();
        }
        let data = self.block_data(pos);
        if block.is_trapdoor() {
            return if self.set_block_with_data(pos, block, data ^ TRAPDOOR_OPEN) {
                vec![pos]
            } else {
                Vec::new()
            };
        }
        if block.is_gate() {
            let data = if data & GATE_OPEN != 0 {
                data & !GATE_OPEN
            } else {
                let facing = if block.facing_dir(data & GATE_FACING_MASK) == -look {
                    facing_code(look)
                } else {
                    data & GATE_FACING_MASK
                };
                facing | GATE_OPEN
            };
            return if self.set_block_with_data(pos, block, data) {
                vec![pos]
            } else {
                Vec::new()
            };
        }
        let lower_pos = if block.door_lower().is_some() {
            pos + IVec3::NEG_Y
        } else {
            pos
        };
        let lower = self.block(lower_pos);
        if lower.door_upper().is_none() {
            return Vec::new();
        }
        let data = self.block_data(lower_pos) ^ DOOR_OPEN;
        self.set_block_with_data(lower_pos, lower, data);
        vec![lower_pos, lower_pos + IVec3::Y]
    }
}

#[cfg(test)]
mod tests;

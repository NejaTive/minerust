use glam::IVec3;

use crate::block::Block;
use crate::chest::{
    CHEST_SLOTS, FACING_MASK, HALF_LEFT, HALF_NONE, HALF_RIGHT, HALF_SHIFT, front_of, half_of,
    right_of,
};
use crate::pos::ColumnPos;
use crate::world::World;

const HORIZONTAL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

impl World {
    fn is_chest(&self, pos: IVec3) -> bool {
        self.block(pos) == Block::Chest
    }

    /// Вторая половина двойного сундука, если она на месте.
    pub fn chest_partner(&self, pos: IVec3) -> Option<IVec3> {
        if !self.is_chest(pos) {
            return None;
        }
        let data = self.block_data(pos);
        let half = half_of(data);
        let right = right_of(front_of(data));
        let partner = match half {
            HALF_LEFT => pos + right,
            HALF_RIGHT => pos - right,
            _ => return None,
        };
        let other = self.block_data(partner);
        (self.is_chest(partner)
            && other & FACING_MASK == data & FACING_MASK
            && half_of(other)
                == if half == HALF_LEFT {
                    HALF_RIGHT
                } else {
                    HALF_LEFT
                })
        .then_some(partner)
    }

    /// Клетки сундука для окна: верхняя половина — сундук с меньшей координатой, как в 1.12.
    pub fn chest_cells(&self, pos: IVec3) -> Vec<IVec3> {
        match self.chest_partner(pos) {
            Some(partner) if (partner.x, partner.z) < (pos.x, pos.z) => vec![partner, pos],
            Some(partner) => vec![pos, partner],
            None => vec![pos],
        }
    }

    /// Над сундуком (или любой его половиной) полный непрозрачный блок: окно не открывается.
    pub fn chest_blocked(&self, pos: IVec3) -> bool {
        self.chest_cells(pos).iter().any(|&cell| {
            let above = self.block(cell + IVec3::Y);
            above.is_opaque() && above.is_solid()
        })
    }

    pub fn chest_contents(&self, pos: IVec3) -> Vec<Option<String>> {
        if !self.is_chest(pos) {
            return Vec::new();
        }
        self.chest_cells(pos)
            .into_iter()
            .flat_map(|cell| {
                let key = ColumnPos::of_block(cell);
                let mut slots = self
                    .column(key)
                    .and_then(|column| column.chest(local_of(cell)))
                    .map_or_else(|| vec![None; CHEST_SLOTS], <[_]>::to_vec);
                slots.resize(CHEST_SLOTS, None);
                slots
            })
            .collect()
    }

    pub fn set_chest_slot(&mut self, pos: IVec3, index: usize, item: Option<String>) -> bool {
        let cells = self.chest_cells(pos);
        let Some(&cell) = cells.get(index / CHEST_SLOTS) else {
            return false;
        };
        let key = ColumnPos::of_block(cell);
        let Some(column) = self.column_mut(key) else {
            return false;
        };
        column.chest_or_create(local_of(cell))[index % CHEST_SLOTS] = item;
        self.mark_edited(key);
        true
    }

    /// `data` нового сундука: лицом к игроку, у соседнего одиночного сундука — его лицо.
    /// `None`, если рядом больше одного сундука или он уже часть двойного, как `BlockChest.canPlaceBlockAt`.
    pub fn chest_placement_data(&self, pos: IVec3, look: IVec3) -> Option<u8> {
        let neighbors: Vec<IVec3> = HORIZONTAL
            .iter()
            .map(|&dir| pos + dir)
            .filter(|&cell| self.is_chest(cell))
            .collect();
        if neighbors.len() > 1
            || neighbors
                .iter()
                .any(|&cell| self.chest_partner(cell).is_some())
        {
            return None;
        }
        let mine = crate::chest::data(-look, HALF_NONE);
        let Some(&neighbor) = neighbors.first() else {
            return Some(mine);
        };
        let front = front_of(self.block_data(neighbor));
        let along = right_of(front);
        let sideways = neighbor - pos == along || neighbor - pos == -along;
        Some(if sideways {
            crate::chest::data(front, HALF_NONE)
        } else {
            mine
        })
    }

    pub(crate) fn link_chest(&mut self, pos: IVec3) {
        let data = self.block_data(pos);
        if half_of(data) != HALF_NONE {
            return;
        }
        let right = right_of(front_of(data));
        for (dir, mine, theirs) in [
            (right, HALF_LEFT, HALF_RIGHT),
            (-right, HALF_RIGHT, HALF_LEFT),
        ] {
            let other = pos + dir;
            let other_data = self.block_data(other);
            if self.is_chest(other)
                && half_of(other_data) == HALF_NONE
                && other_data & FACING_MASK == data & FACING_MASK
            {
                self.rewrite_data(pos, data | mine << HALF_SHIFT);
                self.rewrite_data(other, other_data | theirs << HALF_SHIFT);
                return;
            }
        }
    }

    pub(crate) fn unlink_chest(&mut self, partner: IVec3) {
        let data = self.block_data(partner);
        if self.is_chest(partner) && half_of(data) != HALF_NONE {
            self.rewrite_data(partner, data & FACING_MASK);
        }
    }

    /// Сажает растение в пустой горшок; `false`, если горшок занят или растения нет в списке.
    pub fn plant_in_pot(&mut self, pos: IVec3, plant: Block) -> bool {
        let Some(code) = Block::pot_code(plant) else {
            return false;
        };
        if self.block(pos) != Block::FlowerPot || self.block_data(pos) != 0 {
            return false;
        }
        self.set_block_with_data(pos, Block::FlowerPot, code)
    }
}

fn local_of(cell: IVec3) -> IVec3 {
    crate::world::split(cell).1
}

#[cfg(test)]
mod tests;

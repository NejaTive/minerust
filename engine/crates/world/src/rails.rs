use glam::IVec3;

use crate::block::{
    Block, RAIL_ASCENDING_EAST, RAIL_ASCENDING_NORTH, RAIL_ASCENDING_SOUTH, RAIL_ASCENDING_WEST,
    RAIL_EAST_WEST, RAIL_NORTH_EAST, RAIL_NORTH_SOUTH, RAIL_NORTH_WEST, RAIL_SHAPES,
    RAIL_SOUTH_EAST, RAIL_SOUTH_WEST,
};
use crate::world::World;

const NORTH: IVec3 = IVec3::NEG_Z;
const SOUTH: IVec3 = IVec3::Z;
const WEST: IVec3 = IVec3::NEG_X;
const EAST: IVec3 = IVec3::X;

fn connections(pos: IVec3, shape: u8) -> Vec<IVec3> {
    let at = |dir: IVec3| pos + dir;
    let up = |dir: IVec3| pos + dir + IVec3::Y;
    match shape {
        RAIL_EAST_WEST => vec![at(WEST), at(EAST)],
        RAIL_ASCENDING_EAST => vec![at(WEST), up(EAST)],
        RAIL_ASCENDING_WEST => vec![up(WEST), at(EAST)],
        RAIL_ASCENDING_NORTH => vec![up(NORTH), at(SOUTH)],
        RAIL_ASCENDING_SOUTH => vec![at(NORTH), up(SOUTH)],
        RAIL_SOUTH_EAST => vec![at(EAST), at(SOUTH)],
        RAIL_SOUTH_WEST => vec![at(WEST), at(SOUTH)],
        RAIL_NORTH_WEST => vec![at(WEST), at(NORTH)],
        RAIL_NORTH_EAST => vec![at(EAST), at(NORTH)],
        _ => vec![at(NORTH), at(SOUTH)],
    }
}

fn same_column(a: IVec3, b: IVec3) -> bool {
    a.x == b.x && a.z == b.z
}

/// Рельс и его соединения, как `BlockRailBase.Rail`.
struct Rail {
    pos: IVec3,
    connected: Vec<IVec3>,
}

impl Rail {
    fn is_connected_to(&self, other: IVec3) -> bool {
        self.connected.iter().any(|&c| same_column(c, other))
    }

    fn can_connect_to(&self, other: IVec3) -> bool {
        self.is_connected_to(other) || self.connected.len() != 2
    }
}

impl World {
    fn rail_at(&self, pos: IVec3) -> Option<Rail> {
        [pos, pos + IVec3::Y, pos + IVec3::NEG_Y]
            .into_iter()
            .find(|&p| self.block(p).is_rail())
            .map(|found| {
                let shape = self.block_data(found).min(RAIL_SHAPES - 1);
                Rail {
                    pos: found,
                    connected: connections(found, shape),
                }
            })
    }

    fn drop_soft_connections(&self, rail: &mut Rail) {
        let current = std::mem::take(&mut rail.connected);
        rail.connected = current
            .into_iter()
            .filter_map(|c| {
                let other = self.rail_at(c)?;
                other.is_connected_to(rail.pos).then_some(other.pos)
            })
            .collect();
    }

    fn neighbor_rail(&self, pos: IVec3, this: IVec3) -> bool {
        let Some(mut rail) = self.rail_at(pos) else {
            return false;
        };
        self.drop_soft_connections(&mut rail);
        rail.can_connect_to(this)
    }

    fn is_rail(&self, pos: IVec3) -> bool {
        self.block(pos).is_rail()
    }

    fn lift_straight(&self, pos: IVec3, shape: u8) -> u8 {
        let up = |dir: IVec3| self.is_rail(pos + dir + IVec3::Y);
        let mut shape = shape;
        if shape == RAIL_NORTH_SOUTH {
            if up(NORTH) {
                shape = RAIL_ASCENDING_NORTH;
            }
            if up(SOUTH) {
                shape = RAIL_ASCENDING_SOUTH;
            }
        }
        if shape == RAIL_EAST_WEST {
            if up(EAST) {
                shape = RAIL_ASCENDING_EAST;
            }
            if up(WEST) {
                shape = RAIL_ASCENDING_WEST;
            }
        }
        shape
    }

    /// Выбирает форму рельса в `pos` по соседям и перестраивает их; возвращает изменённые клетки.
    pub fn connect_rail(&mut self, pos: IVec3) -> Vec<IVec3> {
        let (n, s, w, e) = (
            self.neighbor_rail(pos + NORTH, pos),
            self.neighbor_rail(pos + SOUTH, pos),
            self.neighbor_rail(pos + WEST, pos),
            self.neighbor_rail(pos + EAST, pos),
        );
        let mut shape = None;
        if (n || s) && !w && !e {
            shape = Some(RAIL_NORTH_SOUTH);
        }
        if (w || e) && !n && !s {
            shape = Some(RAIL_EAST_WEST);
        }
        for (hit, turn) in [
            (s && e && !n && !w, RAIL_SOUTH_EAST),
            (s && w && !n && !e, RAIL_SOUTH_WEST),
            (n && e && !s && !w, RAIL_NORTH_EAST),
            (n && w && !s && !e, RAIL_NORTH_WEST),
        ] {
            if hit {
                shape = Some(turn);
            }
        }
        if shape.is_none() {
            if n || s {
                shape = Some(RAIL_NORTH_SOUTH);
            }
            if w || e {
                shape = Some(RAIL_EAST_WEST);
            }
            for (hit, turn) in [
                (s && e, RAIL_SOUTH_EAST),
                (w && s, RAIL_SOUTH_WEST),
                (e && n, RAIL_NORTH_EAST),
                (n && w, RAIL_NORTH_WEST),
            ] {
                if hit {
                    shape = Some(turn);
                }
            }
        }
        let shape = self.lift_straight(pos, shape.unwrap_or(RAIL_NORTH_SOUTH));
        self.set_block_with_data(pos, Block::Rail, shape);
        let mut changed = vec![pos];
        for target in connections(pos, shape) {
            let Some(mut rail) = self.rail_at(target) else {
                continue;
            };
            self.drop_soft_connections(&mut rail);
            if rail.can_connect_to(pos) {
                self.join_rail(rail, pos, &mut changed);
            }
        }
        changed
    }

    fn join_rail(&mut self, mut rail: Rail, other: IVec3, changed: &mut Vec<IVec3>) {
        rail.connected.push(other);
        let has = |dir: IVec3| rail.is_connected_to(rail.pos + dir);
        let (n, s, w, e) = (has(NORTH), has(SOUTH), has(WEST), has(EAST));
        let mut shape = None;
        if n || s {
            shape = Some(RAIL_NORTH_SOUTH);
        }
        if w || e {
            shape = Some(RAIL_EAST_WEST);
        }
        for (hit, turn) in [
            (s && e && !n && !w, RAIL_SOUTH_EAST),
            (s && w && !n && !e, RAIL_SOUTH_WEST),
            (n && w && !s && !e, RAIL_NORTH_WEST),
            (n && e && !s && !w, RAIL_NORTH_EAST),
        ] {
            if hit {
                shape = Some(turn);
            }
        }
        let shape = self.lift_straight(rail.pos, shape.unwrap_or(RAIL_NORTH_SOUTH));
        if self.set_block_with_data(rail.pos, Block::Rail, shape) {
            changed.push(rail.pos);
        }
    }
}

#[cfg(test)]
mod tests;

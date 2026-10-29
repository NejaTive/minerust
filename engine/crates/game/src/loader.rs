use std::collections::HashSet;

use glam::IVec2;

use world::column::Column;
use world::pos::ColumnPos;
use world::world::World;

pub const MIN_RADIUS: i32 = 4;
pub const MAX_RADIUS: i32 = 16;
pub const DEFAULT_RADIUS: i32 = 8;

pub const NEIGHBORS: [IVec2; 8] = [
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(1, 1),
    IVec2::new(-1, 1),
    IVec2::new(1, -1),
    IVec2::new(-1, -1),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    Generated(ColumnPos),
    Mesh(ColumnPos),
    Done,
}

pub struct Loader {
    center: ColumnPos,
    radius: i32,
    shrunk: bool,
    offsets: Vec<IVec2>,
    meshed: HashSet<ColumnPos>,
    meshed_prefix: usize,
    loaded_prefix: usize,
}

impl Loader {
    pub fn new(center: ColumnPos) -> Self {
        let mut loader = Self {
            center,
            radius: DEFAULT_RADIUS,
            shrunk: false,
            offsets: Vec::new(),
            meshed: HashSet::new(),
            meshed_prefix: 0,
            loaded_prefix: 0,
        };
        loader.offsets = circle_offsets(loader.gen_radius());
        loader
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }

    pub fn set_radius(&mut self, radius: i32) {
        let radius = radius.clamp(MIN_RADIUS, MAX_RADIUS);
        if radius != self.radius {
            self.shrunk |= radius < self.radius;
            self.radius = radius;
            self.offsets = circle_offsets(self.gen_radius());
            self.reset_prefixes();
        }
    }

    fn gen_radius(&self) -> i32 {
        self.radius + 2
    }

    fn unmesh_radius(&self) -> i32 {
        self.radius + 1
    }

    fn unload_radius(&self) -> i32 {
        self.radius + 3
    }

    pub fn warm_up(
        &mut self,
        world: &mut World,
        mut generate: impl FnMut(ColumnPos) -> Column,
        radius: i32,
    ) -> Vec<ColumnPos> {
        assert!(
            radius <= self.radius,
            "warm-up radius {radius} exceeds {}",
            self.radius
        );
        let center = self.center;
        let in_circle = |r: i32| move |o: &&IVec2| o.length_squared() <= r * r;
        for &o in self.offsets.iter().filter(in_circle(radius + 2)) {
            let key = center + o;
            if world.column(key).is_none() {
                load(world, key, &mut generate);
            }
        }
        let mesh: Vec<ColumnPos> = self
            .offsets
            .iter()
            .filter(in_circle(radius))
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .collect();
        self.meshed.extend(&mesh);
        mesh
    }

    pub fn update(&mut self, center: ColumnPos, world: &mut World) -> Vec<ColumnPos> {
        self.center = center;
        self.reset_prefixes();
        let dist2 = |key: ColumnPos| (key - center).length_squared();

        let shrunk = std::mem::take(&mut self.shrunk);
        let unmesh_radius = self.unmesh_radius() - i32::from(shrunk);
        let mut unmesh = Vec::new();
        self.meshed.retain(|&key| {
            let keep = dist2(key) <= unmesh_radius * unmesh_radius;
            if !keep {
                unmesh.push(key);
            }
            keep
        });
        let unload_radius = self.unload_radius() - i32::from(shrunk);
        let far: Vec<ColumnPos> = world
            .keys()
            .filter(|&key| dist2(key) > unload_radius * unload_radius)
            .collect();
        for key in far {
            world.remove(key);
        }
        unmesh
    }

    pub fn advance(
        &mut self,
        world: &mut World,
        generate: impl FnOnce(ColumnPos) -> Column,
    ) -> Progress {
        if let Some(key) = self.next_to_mesh(world) {
            self.meshed.insert(key);
            return Progress::Mesh(key);
        }
        if let Some(key) = self.next_to_generate(world) {
            load(world, key, generate);
            return Progress::Generated(key);
        }
        Progress::Done
    }

    pub fn pending(&self) -> usize {
        self.offsets[..self.circle_len(self.radius)]
            .iter()
            .filter(|&&o| !self.meshed.contains(&(self.center + o)))
            .count()
    }

    fn next_to_mesh(&mut self, world: &World) -> Option<ColumnPos> {
        let center = self.center;
        let circle = &self.offsets[..self.circle_len(self.radius)];
        while circle
            .get(self.meshed_prefix)
            .is_some_and(|&o| self.meshed.contains(&(center + o)))
        {
            self.meshed_prefix += 1;
        }
        let loaded = |key: ColumnPos| world.column(key).is_some();
        circle[self.meshed_prefix..]
            .iter()
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .find(|&key| loaded(key) && NEIGHBORS.iter().all(|&n| loaded(key + n)))
    }

    fn next_to_generate(&mut self, world: &World) -> Option<ColumnPos> {
        let center = self.center;
        let missing = self.offsets[self.loaded_prefix..]
            .iter()
            .position(|&o| world.column(center + o).is_none());
        match missing {
            Some(i) => {
                self.loaded_prefix += i;
                Some(center + self.offsets[self.loaded_prefix])
            }
            None => {
                self.loaded_prefix = self.offsets.len();
                None
            }
        }
    }

    fn reset_prefixes(&mut self) {
        self.meshed_prefix = 0;
        self.loaded_prefix = 0;
    }

    fn circle_len(&self, radius: i32) -> usize {
        self.offsets
            .partition_point(|o| o.length_squared() <= radius * radius)
    }
}

fn load(world: &mut World, key: ColumnPos, generate: impl FnOnce(ColumnPos) -> Column) {
    let column = world.take_edited(key).unwrap_or_else(|| generate(key));
    world.insert(key, column);
}

fn circle_offsets(radius: i32) -> Vec<IVec2> {
    let r = radius;
    let mut offsets: Vec<IVec2> = (-r..=r)
        .flat_map(|z| (-r..=r).map(move |x| IVec2::new(x, z)))
        .filter(|o| o.length_squared() <= r * r)
        .collect();
    offsets.sort_by_key(|o| (o.length_squared(), o.y, o.x));
    offsets
}

#[cfg(test)]
mod tests;

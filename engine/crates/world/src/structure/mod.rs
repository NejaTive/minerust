use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use glam::IVec3;

use crate::column::Column;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::stream::Stream;
use crate::terrain::Terrain;

mod cell;
mod igloo;
mod piece;
mod placement;
mod showcase;
mod swamp_hut;
mod template;

pub use piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter};
pub use showcase::{
    FIRST_PLOT_X, Ground, LAND_SURFACE, Plot, SEABED_SURFACE, biome as plot_biome, plot_at, plots,
    surface as plot_surface,
};

const START_CACHE_LIMIT: usize = 1024;
const SALT_START: u64 = 400;
const MIN_LOCATE_REACH: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum StructureKind {
    Village,
    Temple,
    Mineshaft,
    Stronghold,
    Monument,
    Mansion,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TempleKind {
    DesertPyramid,
    JunglePyramid,
    SwampHut,
    Igloo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MineshaftKind {
    Normal,
    Mesa,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Variant {
    Plain,
    Temple(TempleKind),
    Mineshaft(MineshaftKind),
}

impl StructureKind {
    pub const ALL: [StructureKind; 6] = [
        StructureKind::Village,
        StructureKind::Temple,
        StructureKind::Mineshaft,
        StructureKind::Stronghold,
        StructureKind::Monument,
        StructureKind::Mansion,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StructureKind::Village => "Village",
            StructureKind::Temple => "Temple",
            StructureKind::Mineshaft => "Mineshaft",
            StructureKind::Stronghold => "Stronghold",
            StructureKind::Monument => "Monument",
            StructureKind::Mansion => "Mansion",
        }
    }

    /// На сколько колонок от старта могут уходить части.
    pub fn reach(self) -> i32 {
        match self {
            StructureKind::Village => 8,
            StructureKind::Temple => 2,
            StructureKind::Mineshaft => 8,
            StructureKind::Stronghold => 9,
            StructureKind::Monument => 4,
            StructureKind::Mansion => 8,
        }
    }

    /// Подземные виды не мешают поверхностному декору.
    pub fn underground(self) -> bool {
        matches!(self, StructureKind::Mineshaft | StructureKind::Stronghold)
    }

    fn index(self) -> u8 {
        self as u8
    }
}

impl Variant {
    /// Варианты без построек не создаются: старт не строится, `/locate` их не знает.
    pub fn ready(self) -> bool {
        match self {
            Variant::Temple(kind) => kind.ready(),
            Variant::Plain | Variant::Mineshaft(_) => true,
        }
    }
}

impl TempleKind {
    pub fn ready(self) -> bool {
        matches!(self, TempleKind::SwampHut | TempleKind::Igloo)
    }

    pub fn name(self) -> &'static str {
        match self {
            TempleKind::DesertPyramid => "Desert_Pyramid",
            TempleKind::JunglePyramid => "Jungle_Pyramid",
            TempleKind::SwampHut => "Swamp_Hut",
            TempleKind::Igloo => "Igloo",
        }
    }
}

/// Имя для `/locate`: вид целиком или его точный вариант.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LocateName {
    pub name: &'static str,
    pub kind: StructureKind,
    pub variant: Option<Variant>,
}

pub const LOCATE_NAMES: [LocateName; 10] = [
    LocateName::of("Village", StructureKind::Village, None),
    LocateName::of("Temple", StructureKind::Temple, None),
    LocateName::of("Mineshaft", StructureKind::Mineshaft, None),
    LocateName::of("Stronghold", StructureKind::Stronghold, None),
    LocateName::of("Monument", StructureKind::Monument, None),
    LocateName::of("Mansion", StructureKind::Mansion, None),
    LocateName::of(
        "Desert_Pyramid",
        StructureKind::Temple,
        Some(Variant::Temple(TempleKind::DesertPyramid)),
    ),
    LocateName::of(
        "Jungle_Pyramid",
        StructureKind::Temple,
        Some(Variant::Temple(TempleKind::JunglePyramid)),
    ),
    LocateName::of(
        "Swamp_Hut",
        StructureKind::Temple,
        Some(Variant::Temple(TempleKind::SwampHut)),
    ),
    LocateName::of(
        "Igloo",
        StructureKind::Temple,
        Some(Variant::Temple(TempleKind::Igloo)),
    ),
];

impl LocateName {
    const fn of(name: &'static str, kind: StructureKind, variant: Option<Variant>) -> Self {
        Self {
            name,
            kind,
            variant,
        }
    }

    pub fn ready(self) -> bool {
        self.variant.is_none_or(Variant::ready)
    }

    pub fn find(name: &str) -> Option<LocateName> {
        LOCATE_NAMES
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug)]
pub struct StructureStart {
    pub kind: StructureKind,
    pub variant: Variant,
    pub origin: ColumnPos,
    pub anchor: (i32, i32),
    pub pieces: Vec<Piece>,
    pub bounds: BlockBox,
}

impl StructureStart {
    pub fn reaches(&self, key: ColumnPos) -> bool {
        self.bounds.intersects_xz(&BlockBox::of_column(key))
            && self
                .pieces
                .iter()
                .any(|piece| piece.bounds.intersects_xz(&BlockBox::of_column(key)))
    }

    pub fn covers(&self, x: i32, z: i32) -> bool {
        self.bounds.contains_xz(x, z)
            && self
                .pieces
                .iter()
                .any(|piece| piece.bounds.contains_xz(x, z))
    }
}

pub fn build(terrain: &Terrain, kind: StructureKind, origin: ColumnPos) -> Option<StructureStart> {
    let variant = placement::variant(terrain, kind, origin).filter(|variant| variant.ready())?;
    let anchor = placement::anchor(terrain, origin);
    let mut rng = placement::stream(
        terrain.seed(),
        origin.x(),
        origin.z(),
        SALT_START + 2 * u64::from(kind.index()),
    );
    let pieces = layout(terrain, kind, variant, anchor, &mut rng);
    let probe = IVec3::new(anchor.0, 0, anchor.1);
    let bounds = pieces
        .iter()
        .map(|piece| piece.bounds)
        .reduce(|a, b| a.union(&b))
        .unwrap_or(BlockBox::new(probe, probe));
    Some(StructureStart {
        kind,
        variant,
        origin,
        anchor,
        pieces,
        bounds,
    })
}

fn layout(
    terrain: &Terrain,
    kind: StructureKind,
    variant: Variant,
    anchor: (i32, i32),
    rng: &mut Stream,
) -> Vec<Piece> {
    match variant {
        Variant::Temple(TempleKind::SwampHut) => swamp_hut::layout(terrain, anchor, rng),
        Variant::Temple(TempleKind::Igloo) => igloo::layout(terrain, anchor, rng),
        #[cfg(test)]
        _ => tests::layout(terrain, kind, variant, anchor, rng),
        #[cfg(not(test))]
        _ => {
            let _ = (kind, anchor, rng);
            Vec::new()
        }
    }
}

type StartCache = HashMap<(StructureKind, i32, i32), Arc<Option<StructureStart>>>;

pub struct Structures {
    built: AtomicU8,
    cache: Mutex<StartCache>,
    strongholds: OnceLock<Vec<ColumnPos>>,
}

impl Default for Structures {
    fn default() -> Self {
        Self {
            built: AtomicU8::new(1 << StructureKind::Temple.index()),
            cache: Mutex::default(),
            strongholds: OnceLock::new(),
        }
    }
}

impl Structures {
    pub(crate) fn built(&self, kind: StructureKind) -> bool {
        self.built.load(Ordering::Relaxed) & (1 << kind.index()) != 0
    }

    pub(crate) fn set_built(&self, kind: StructureKind, on: bool) {
        let bit = 1 << kind.index();
        if on {
            self.built.fetch_or(bit, Ordering::Relaxed);
        } else {
            self.built.fetch_and(!bit, Ordering::Relaxed);
        }
        self.cache.lock().expect("structure cache poisoned").clear();
    }

    pub(crate) fn strongholds(&self, terrain: &Terrain) -> &[ColumnPos] {
        self.strongholds
            .get_or_init(|| placement::stronghold_ring(terrain))
    }

    fn start(&self, terrain: &Terrain, kind: StructureKind, origin: ColumnPos) -> StartRef {
        let key = (kind, origin.x(), origin.z());
        if let Some(found) = self
            .cache
            .lock()
            .expect("structure cache poisoned")
            .get(&key)
        {
            return StartRef(Arc::clone(found));
        }
        let built = Arc::new(build(terrain, kind, origin));
        let mut cache = self.cache.lock().expect("structure cache poisoned");
        if cache.len() >= START_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, Arc::clone(&built));
        StartRef(built)
    }
}

/// Построенный старт из кэша; `None` внутри — место не прошло проверку биома.
#[derive(Clone)]
pub struct StartRef(Arc<Option<StructureStart>>);

impl StartRef {
    pub fn get(&self) -> Option<&StructureStart> {
        self.0.as_ref().as_ref()
    }
}

impl Terrain {
    pub fn structure_built(&self, kind: StructureKind) -> bool {
        self.structures().built(kind)
    }

    /// Включает вид в генерацию и `/locate`; пока у вида нет построек, флаг выключен.
    #[doc(hidden)]
    pub fn set_structure_built(&self, kind: StructureKind, on: bool) {
        self.structures().set_built(kind, on);
    }

    /// Старты вида в квадрате `reach` колонок вокруг `center`, прошедшие проверку биома.
    pub fn starts_near(&self, kind: StructureKind, center: ColumnPos, reach: i32) -> Vec<StartRef> {
        if !self.structure_built(kind) {
            return Vec::new();
        }
        placement::origins(self, kind, center, reach)
            .into_iter()
            .map(|origin| self.structures().start(self, kind, origin))
            .filter(|start| start.get().is_some())
            .collect()
    }

    pub fn starts_touching(&self, key: ColumnPos) -> Vec<StartRef> {
        StructureKind::ALL
            .into_iter()
            .flat_map(|kind| self.starts_near(kind, key, kind.reach()))
            .filter(|start| start.get().is_some_and(|start| start.reaches(key)))
            .collect()
    }

    pub(crate) fn place_structures(&self, key: ColumnPos, column: &mut Column) {
        for start in self.starts_touching(key) {
            let Some(start) = start.get() else { continue };
            let frame = BlockBox::of_column(key);
            for piece in &start.pieces {
                if piece.bounds.intersects_xz(&frame) {
                    piece.place(&mut PieceWriter::new(column, key, piece));
                }
            }
        }
    }

    /// Стоит ли над (x, z) наземная структура: решение по рамкам частей, не зависит от содержимого колонок.
    pub fn structure_covers(&self, x: i32, z: i32) -> bool {
        let key = ColumnPos::new(x >> 4, z >> 4);
        StructureKind::ALL
            .into_iter()
            .filter(|kind| !kind.underground())
            .any(|kind| {
                self.starts_near(kind, key, kind.reach())
                    .iter()
                    .any(|start| start.get().is_some_and(|start| start.covers(x, z)))
            })
    }

    pub(crate) fn village_touches(&self, key: ColumnPos) -> bool {
        let kind = StructureKind::Village;
        self.starts_near(kind, key, kind.reach())
            .iter()
            .any(|start| start.get().is_some_and(|start| start.reaches(key)))
    }

    /// Ближайший старт вида в радиусе `radius` блоков: `(x, z, расстояние)` его якоря.
    pub fn nearest_structure(
        &self,
        target: LocateName,
        x: i32,
        z: i32,
        radius: i32,
    ) -> Option<(i32, i32, i32)> {
        let center = ColumnPos::new(x >> 4, z >> 4);
        let mut reach = target.kind.reach().max(MIN_LOCATE_REACH);
        let limit = radius / SECTION_SIZE + 1;
        loop {
            reach = reach.min(limit);
            let best = self
                .starts_near(target.kind, center, reach)
                .iter()
                .filter_map(|start| start.get())
                .filter(|start| {
                    target
                        .variant
                        .is_none_or(|variant| start.variant == variant)
                })
                .map(|start| {
                    let d2 =
                        i64::from(start.anchor.0 - x).pow(2) + i64::from(start.anchor.1 - z).pow(2);
                    (d2, start.anchor)
                })
                .min();
            let covered = i64::from(reach * SECTION_SIZE);
            let found = best.filter(|&(d2, _)| d2 <= covered * covered);
            if let Some((d2, (ax, az))) = found {
                let distance = (d2 as f64).sqrt().round() as i32;
                return (distance <= radius).then_some((ax, az, distance));
            }
            if reach >= limit {
                return None;
            }
            reach *= 2;
        }
    }
}

#[cfg(test)]
mod tests;

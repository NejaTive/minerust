use glam::Vec3;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobKind {
    Pig,
    Chicken,
    Cow,
    Sheep,
    Mooshroom,
    PolarBear,
    Wolf,
    Horse,
    Donkey,
    Llama,
    Rabbit,
    Ocelot,
}

impl MobKind {
    pub const ALL: [MobKind; 12] = [
        MobKind::Pig,
        MobKind::Chicken,
        MobKind::Cow,
        MobKind::Sheep,
        MobKind::Mooshroom,
        MobKind::PolarBear,
        MobKind::Wolf,
        MobKind::Horse,
        MobKind::Donkey,
        MobKind::Llama,
        MobKind::Rabbit,
        MobKind::Ocelot,
    ];
}

impl TryFrom<u8> for MobKind {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Self::ALL
            .into_iter()
            .find(|kind| *kind as u8 == value)
            .ok_or(value)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MobRecord {
    pub kind: MobKind,
    pub pos: Vec3,
    pub yaw: f32,
    pub variant: u8,
}

pub const HORSE_COLOURS: u8 = 7;
pub const HORSE_MARKINGS: u8 = 5;
pub const LLAMA_COLOURS: u8 = 4;

pub const RABBIT_BROWN: u8 = 0;
pub const RABBIT_WHITE: u8 = 1;
pub const RABBIT_BLACK: u8 = 2;
pub const RABBIT_SPLOTCHED: u8 = 3;
pub const RABBIT_GOLD: u8 = 4;
pub const RABBIT_SALT: u8 = 5;
pub const RABBIT_COLOURS: u8 = 6;

/// Вариант лошади: младшие 4 бита — окрас (0..7), старшие — отметины (0 — нет, 1..=4).
pub const fn horse_variant(colour: u8, markings: u8) -> u8 {
    colour | markings << 4
}

pub const fn horse_colour(variant: u8) -> u8 {
    variant & 0x0F
}

pub const fn horse_markings(variant: u8) -> u8 {
    variant >> 4
}

#[cfg(test)]
mod tests;

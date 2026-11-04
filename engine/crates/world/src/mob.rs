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
}

impl MobKind {
    pub const ALL: [MobKind; 7] = [
        MobKind::Pig,
        MobKind::Chicken,
        MobKind::Cow,
        MobKind::Sheep,
        MobKind::Mooshroom,
        MobKind::PolarBear,
        MobKind::Wolf,
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
}

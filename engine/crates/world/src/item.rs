use crate::mob::MobKind;
use crate::pack::EntityImages;
use crate::textures::{DecodedImage, scale_nearest};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    PigSpawnEgg,
    ChickenSpawnEgg,
    CowSpawnEgg,
    SheepSpawnEgg,
    MooshroomSpawnEgg,
    PolarBearSpawnEgg,
    WolfSpawnEgg,
    HorseSpawnEgg,
    DonkeySpawnEgg,
    LlamaSpawnEgg,
}

pub struct ItemDef {
    pub name: &'static str,
    pub label: &'static str,
    pub spawns: MobKind,
    pub egg_primary: [u8; 3],
    pub egg_secondary: [u8; 3],
}

pub const ITEMS: [ItemDef; 10] = [
    ItemDef {
        name: "pig_spawn_egg",
        label: "яйцо призыва свиньи",
        spawns: MobKind::Pig,
        egg_primary: [0xF0, 0xA5, 0xA2],
        egg_secondary: [0xDB, 0x63, 0x5F],
    },
    ItemDef {
        name: "chicken_spawn_egg",
        label: "яйцо призыва курицы",
        spawns: MobKind::Chicken,
        egg_primary: [0xA1, 0xA1, 0xA1],
        egg_secondary: [0xFF, 0x00, 0x00],
    },
    ItemDef {
        name: "cow_spawn_egg",
        label: "яйцо призыва коровы",
        spawns: MobKind::Cow,
        egg_primary: [0x44, 0x36, 0x26],
        egg_secondary: [0xA1, 0xA1, 0xA1],
    },
    ItemDef {
        name: "sheep_spawn_egg",
        label: "яйцо призыва овцы",
        spawns: MobKind::Sheep,
        egg_primary: [0xE7, 0xE7, 0xE7],
        egg_secondary: [0xFF, 0xB5, 0xB5],
    },
    ItemDef {
        name: "mooshroom_spawn_egg",
        label: "яйцо призыва грибной коровы",
        spawns: MobKind::Mooshroom,
        egg_primary: [0xA0, 0x0F, 0x10],
        egg_secondary: [0xB7, 0xB7, 0xB7],
    },
    ItemDef {
        name: "polar_bear_spawn_egg",
        label: "яйцо призыва белого медведя",
        spawns: MobKind::PolarBear,
        egg_primary: [0xF2, 0xF2, 0xF2],
        egg_secondary: [0x95, 0x95, 0x90],
    },
    ItemDef {
        name: "wolf_spawn_egg",
        label: "яйцо призыва волка",
        spawns: MobKind::Wolf,
        egg_primary: [0xD7, 0xD3, 0xD3],
        egg_secondary: [0xCE, 0xAF, 0x96],
    },
    ItemDef {
        name: "horse_spawn_egg",
        label: "яйцо призыва лошади",
        spawns: MobKind::Horse,
        egg_primary: [0xC0, 0x9E, 0x7D],
        egg_secondary: [0xEE, 0xE5, 0x00],
    },
    ItemDef {
        name: "donkey_spawn_egg",
        label: "яйцо призыва осла",
        spawns: MobKind::Donkey,
        egg_primary: [0x53, 0x45, 0x39],
        egg_secondary: [0x86, 0x75, 0x66],
    },
    ItemDef {
        name: "llama_spawn_egg",
        label: "яйцо призыва ламы",
        spawns: MobKind::Llama,
        egg_primary: [0xC0, 0x9E, 0x7D],
        egg_secondary: [0x99, 0x5F, 0x40],
    },
];

impl Item {
    pub const ALL: [Item; 10] = [
        Item::PigSpawnEgg,
        Item::ChickenSpawnEgg,
        Item::CowSpawnEgg,
        Item::SheepSpawnEgg,
        Item::MooshroomSpawnEgg,
        Item::PolarBearSpawnEgg,
        Item::WolfSpawnEgg,
        Item::HorseSpawnEgg,
        Item::DonkeySpawnEgg,
        Item::LlamaSpawnEgg,
    ];

    pub fn def(self) -> &'static ItemDef {
        &ITEMS[self as usize]
    }

    pub fn from_name(name: &str) -> Option<Item> {
        Self::ALL.into_iter().find(|item| item.def().name == name)
    }

    pub fn icon_size(entities: &EntityImages) -> u32 {
        entities.spawn_egg.width
    }

    pub fn icon_pixels(self, entities: &EntityImages) -> Vec<u8> {
        let def = self.def();
        let size = entities.spawn_egg.width;
        let mut pixels = tinted(&entities.spawn_egg, size, def.egg_primary);
        let overlay = tinted(&entities.spawn_egg_overlay, size, def.egg_secondary);
        for (dst, src) in pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(overlay.as_chunks::<4>().0)
        {
            let alpha = f32::from(src[3]) / 255.0;
            for c in 0..3 {
                dst[c] =
                    (f32::from(dst[c]) * (1.0 - alpha) + f32::from(src[c]) * alpha).round() as u8;
            }
            dst[3] = dst[3].max(src[3]);
        }
        pixels
    }
}

fn tinted(image: &DecodedImage, size: u32, color: [u8; 3]) -> Vec<u8> {
    let mut pixels = scale_nearest(&image.rgba, image.width, size);
    for p in pixels.as_chunks_mut::<4>().0 {
        for c in 0..3 {
            p[c] = (u16::from(p[c]) * u16::from(color[c]) / 255) as u8;
        }
    }
    pixels
}

#[cfg(test)]
mod tests;

use std::io::Read;

use serde_json::Value;

use crate::biome_color::{BiomePalette, BiomeTint, Colormap};
use crate::textures::{
    DecodedImage, MAX_LAYERS, TextureSet, TileTexture, WHITE, decode_png, scale_nearest,
};
use crate::tile::Tile;

const BLOCKS_DIR: &str = "assets/minecraft/textures/blocks/";
const ENTITY_PIG: &str = "assets/minecraft/textures/entity/pig/pig.png";
const ENTITY_CHICKEN: &str = "assets/minecraft/textures/entity/chicken.png";
const ENTITY_COW: &str = "assets/minecraft/textures/entity/cow/cow.png";
const ENTITY_MOOSHROOM: &str = "assets/minecraft/textures/entity/cow/mooshroom.png";
const ENTITY_POLAR_BEAR: &str = "assets/minecraft/textures/entity/bear/polarbear.png";
const ENTITY_WOLF: &str = "assets/minecraft/textures/entity/wolf/wolf.png";
const BLOCK_MUSHROOM_RED: &str = "assets/minecraft/textures/blocks/mushroom_red.png";
const ENTITY_HORSE: [&str; 7] = [
    "assets/minecraft/textures/entity/horse/horse_white.png",
    "assets/minecraft/textures/entity/horse/horse_creamy.png",
    "assets/minecraft/textures/entity/horse/horse_chestnut.png",
    "assets/minecraft/textures/entity/horse/horse_brown.png",
    "assets/minecraft/textures/entity/horse/horse_black.png",
    "assets/minecraft/textures/entity/horse/horse_gray.png",
    "assets/minecraft/textures/entity/horse/horse_darkbrown.png",
];
const ENTITY_HORSE_MARKINGS: [&str; 4] = [
    "assets/minecraft/textures/entity/horse/horse_markings_white.png",
    "assets/minecraft/textures/entity/horse/horse_markings_whitefield.png",
    "assets/minecraft/textures/entity/horse/horse_markings_whitedots.png",
    "assets/minecraft/textures/entity/horse/horse_markings_blackdots.png",
];
const ENTITY_DONKEY: &str = "assets/minecraft/textures/entity/horse/donkey.png";
const ENTITY_LLAMA: [&str; 4] = [
    "assets/minecraft/textures/entity/llama/llama_creamy.png",
    "assets/minecraft/textures/entity/llama/llama_white.png",
    "assets/minecraft/textures/entity/llama/llama_brown.png",
    "assets/minecraft/textures/entity/llama/llama_gray.png",
];
const ENTITY_RABBIT: [&str; 6] = [
    "assets/minecraft/textures/entity/rabbit/brown.png",
    "assets/minecraft/textures/entity/rabbit/white.png",
    "assets/minecraft/textures/entity/rabbit/black.png",
    "assets/minecraft/textures/entity/rabbit/white_splotched.png",
    "assets/minecraft/textures/entity/rabbit/gold.png",
    "assets/minecraft/textures/entity/rabbit/salt.png",
];
const ENTITY_PARROT: [&str; 5] = [
    "assets/minecraft/textures/entity/parrot/parrot_red_blue.png",
    "assets/minecraft/textures/entity/parrot/parrot_blue.png",
    "assets/minecraft/textures/entity/parrot/parrot_green.png",
    "assets/minecraft/textures/entity/parrot/parrot_yellow_blue.png",
    "assets/minecraft/textures/entity/parrot/parrot_grey.png",
];
const ENTITY_SQUID: &str = "assets/minecraft/textures/entity/squid.png";
const ENTITY_OCELOT: &str = "assets/minecraft/textures/entity/cat/ocelot.png";
const ENTITY_SHEEP: &str = "assets/minecraft/textures/entity/sheep/sheep.png";
const ENTITY_SHEEP_FUR: &str = "assets/minecraft/textures/entity/sheep/sheep_fur.png";
const ITEM_SPAWN_EGG: &str = "assets/minecraft/textures/items/spawn_egg.png";
const ITEM_SPAWN_EGG_OVERLAY: &str = "assets/minecraft/textures/items/spawn_egg_overlay.png";
const GUI_DIR: &str = "assets/minecraft/textures/gui/";
const GUI_WIDGETS: &str = "widgets.png";
const GUI_ICONS: &str = "icons.png";
const GUI_INVENTORY: &str = "container/inventory.png";
const GUI_SIZE_UNITS: u32 = 256;
const PANEL_CORNER_UNITS: u32 = 4;
const PANEL_WIDTH_UNITS: u32 = 176;
const PANEL_HEIGHT_UNITS: u32 = 166;
const PANEL_FLAT_X_UNITS: u32 = 80;
const PANEL_FLAT_Y_UNITS: u32 = 30;
const PANEL_EDGE_UNITS: u32 = 80;
const COLORMAP_DIR: &str = "assets/minecraft/textures/colormap/";
const MIN_TILE_SIZE: u32 = 16;
const MAX_TILE_SIZE: u32 = 512;
const MAX_ENTRY_BYTES: u64 = 64 << 20;
const WATER_MAX_ALPHA: u8 = 180;

const BIRCH_LEAVES: [f32; 3] = rgb(0x80A755);
const SPRUCE_LEAVES: [f32; 3] = rgb(0x619961);
const WATER: [f32; 3] = rgb(0x3F76E4);
const WATERLILY: [f32; 3] = rgb(0x208030);

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

pub struct TexturePack {
    pub textures: TextureSet,
    pub description: String,
    pub icon: Option<Vec<u8>>,
    pub gui: GuiImages,
    pub entities: EntityImages,
}

#[derive(Clone)]
pub struct EntityImages {
    /// Лист 2:1 (в ванильном паке 64×32); UV модели нормализованы по размеру листа.
    pub pig: DecodedImage,
    pub chicken: DecodedImage,
    pub cow: DecodedImage,
    pub sheep: DecodedImage,
    pub sheep_fur: DecodedImage,
    pub mooshroom: DecodedImage,
    pub polar_bear: DecodedImage,
    pub wolf: DecodedImage,
    /// Листы лошади, осла и ламы квадратные (лошади, 128×128 в ванили) или 2:1 (лама, 128×64).
    pub horse: [DecodedImage; 7],
    pub horse_markings: [DecodedImage; 4],
    pub donkey: DecodedImage,
    pub llama: [DecodedImage; 4],
    /// Листы кролика в порядке окрасов: бурый, белый, чёрный, белый пятнистый, золотой, соль с перцем.
    pub rabbit: [DecodedImage; 6],
    pub ocelot: DecodedImage,
    /// Листы попугая в порядке окрасов: красно-синий, синий, зелёный, жёлто-синий, серый; лист квадратный (32×32 в сетке модели).
    pub parrot: [DecodedImage; 5],
    pub squid: DecodedImage,
    /// Квадратная блочная текстура гриба для скрещённых плоскостей на спине грибной коровы.
    pub mushroom_red: DecodedImage,
    pub spawn_egg: DecodedImage,
    pub spawn_egg_overlay: DecodedImage,
}

/// PNG-листы интерфейса; координаты в них считаются в единицах 256×256 (`width / 256` пикселей на единицу).
#[derive(Clone)]
pub struct GuiImages {
    pub widgets: Vec<u8>,
    pub icons: Vec<u8>,
    pub inventory: Vec<u8>,
    /// Девять квадратов 3×3 из углов, краёв и фона панели инвентаря, для `border-image`.
    pub panel: Vec<u8>,
    pub panel_slice: u32,
}

fn source(tile: Tile) -> (&'static str, [f32; 3], BiomeTint) {
    match tile {
        Tile::GrassTop => ("grass_top", WHITE, BiomeTint::Grass),
        Tile::GrassSide => ("grass_side", WHITE, BiomeTint::None),
        Tile::Dirt => ("dirt", WHITE, BiomeTint::None),
        Tile::Stone => ("stone", WHITE, BiomeTint::None),
        Tile::Sand => ("sand", WHITE, BiomeTint::None),
        Tile::LogSide => ("log_oak", WHITE, BiomeTint::None),
        Tile::LogTop => ("log_oak_top", WHITE, BiomeTint::None),
        Tile::Leaves => ("leaves_oak", WHITE, BiomeTint::Foliage),
        Tile::Water => ("water_still", WATER, BiomeTint::Water),
        Tile::Glowstone => ("glowstone", WHITE, BiomeTint::None),
        Tile::TallGrass => ("tallgrass", WHITE, BiomeTint::Grass),
        Tile::Poppy => ("flower_rose", WHITE, BiomeTint::None),
        Tile::Dandelion => ("flower_dandelion", WHITE, BiomeTint::None),
        Tile::Torch => ("torch_on", WHITE, BiomeTint::None),
        Tile::Gravel => ("gravel", WHITE, BiomeTint::None),
        Tile::Lava => ("lava_still", WHITE, BiomeTint::None),
        Tile::Obsidian => ("obsidian", WHITE, BiomeTint::None),
        Tile::Cobblestone => ("cobblestone", WHITE, BiomeTint::None),
        Tile::BirchLogSide => ("log_birch", WHITE, BiomeTint::None),
        Tile::BirchLogTop => ("log_birch_top", WHITE, BiomeTint::None),
        Tile::BirchLeaves => ("leaves_birch", BIRCH_LEAVES, BiomeTint::None),
        Tile::SpruceLogSide => ("log_spruce", WHITE, BiomeTint::None),
        Tile::SpruceLogTop => ("log_spruce_top", WHITE, BiomeTint::None),
        Tile::SpruceLeaves => ("leaves_spruce", SPRUCE_LEAVES, BiomeTint::None),
        Tile::Bedrock => ("bedrock", WHITE, BiomeTint::None),
        Tile::SnowyGrassSide => ("grass_side_snowed", WHITE, BiomeTint::None),
        Tile::Snow => ("snow", WHITE, BiomeTint::None),
        Tile::Ice => ("ice", WHITE, BiomeTint::None),
        Tile::CactusTop => ("cactus_top", WHITE, BiomeTint::None),
        Tile::CactusSide => ("cactus_side", WHITE, BiomeTint::None),
        Tile::CactusBottom => ("cactus_bottom", WHITE, BiomeTint::None),
        Tile::DeadBush => ("deadbush", WHITE, BiomeTint::None),
        Tile::RedSand => ("red_sand", WHITE, BiomeTint::None),
        Tile::Terracotta => ("hardened_clay", WHITE, BiomeTint::None),
        Tile::TerracottaWhite => ("hardened_clay_stained_white", WHITE, BiomeTint::None),
        Tile::TerracottaOrange => ("hardened_clay_stained_orange", WHITE, BiomeTint::None),
        Tile::TerracottaYellow => ("hardened_clay_stained_yellow", WHITE, BiomeTint::None),
        Tile::TerracottaBrown => ("hardened_clay_stained_brown", WHITE, BiomeTint::None),
        Tile::TerracottaRed => ("hardened_clay_stained_red", WHITE, BiomeTint::None),
        Tile::TerracottaSilver => ("hardened_clay_stained_silver", WHITE, BiomeTint::None),
        Tile::Reeds => ("reeds", WHITE, BiomeTint::Grass),
        Tile::SandstoneTop => ("sandstone_top", WHITE, BiomeTint::None),
        Tile::SandstoneSide => ("sandstone_normal", WHITE, BiomeTint::None),
        Tile::SandstoneBottom => ("sandstone_bottom", WHITE, BiomeTint::None),
        Tile::RedSandstoneTop => ("red_sandstone_top", WHITE, BiomeTint::None),
        Tile::RedSandstoneSide => ("red_sandstone_normal", WHITE, BiomeTint::None),
        Tile::RedSandstoneBottom => ("red_sandstone_bottom", WHITE, BiomeTint::None),
        Tile::JungleLogSide => ("log_jungle", WHITE, BiomeTint::None),
        Tile::JungleLogTop => ("log_jungle_top", WHITE, BiomeTint::None),
        Tile::JungleLeaves => ("leaves_jungle", WHITE, BiomeTint::Foliage),
        Tile::AcaciaLogSide => ("log_acacia", WHITE, BiomeTint::None),
        Tile::AcaciaLogTop => ("log_acacia_top", WHITE, BiomeTint::None),
        Tile::AcaciaLeaves => ("leaves_acacia", WHITE, BiomeTint::Foliage),
        Tile::DarkOakLogSide => ("log_big_oak", WHITE, BiomeTint::None),
        Tile::DarkOakLogTop => ("log_big_oak_top", WHITE, BiomeTint::None),
        Tile::DarkOakLeaves => ("leaves_big_oak", WHITE, BiomeTint::Foliage),
        Tile::Vine => ("vine", WHITE, BiomeTint::Foliage),
        Tile::MelonSide => ("melon_side", WHITE, BiomeTint::None),
        Tile::MelonTop => ("melon_top", WHITE, BiomeTint::None),
        Tile::MushroomSkinBrown => ("mushroom_block_skin_brown", WHITE, BiomeTint::None),
        Tile::MushroomSkinRed => ("mushroom_block_skin_red", WHITE, BiomeTint::None),
        Tile::MushroomStem => ("mushroom_block_skin_stem", WHITE, BiomeTint::None),
        Tile::MushroomInside => ("mushroom_block_inside", WHITE, BiomeTint::None),
        Tile::Waterlily => ("waterlily", WATERLILY, BiomeTint::None),
        Tile::MyceliumTop => ("mycelium_top", WHITE, BiomeTint::None),
        Tile::MyceliumSide => ("mycelium_side", WHITE, BiomeTint::None),
        Tile::BrownMushroom => ("mushroom_brown", WHITE, BiomeTint::None),
        Tile::RedMushroom => ("mushroom_red", WHITE, BiomeTint::None),
        Tile::Clay => ("clay", WHITE, BiomeTint::None),
        Tile::IcePacked => ("ice_packed", WHITE, BiomeTint::None),
        Tile::PodzolTop => ("dirt_podzol_top", WHITE, BiomeTint::None),
        Tile::PodzolSide => ("dirt_podzol_side", WHITE, BiomeTint::None),
        Tile::CoarseDirt => ("coarse_dirt", WHITE, BiomeTint::None),
        Tile::MossyCobblestone => ("cobblestone_mossy", WHITE, BiomeTint::None),
        Tile::Fern => ("fern", WHITE, BiomeTint::Grass),
        Tile::DoubleGrassBottom => ("double_plant_grass_bottom", WHITE, BiomeTint::Grass),
        Tile::DoubleGrassTop => ("double_plant_grass_top", WHITE, BiomeTint::Grass),
        Tile::DoubleFernBottom => ("double_plant_fern_bottom", WHITE, BiomeTint::Grass),
        Tile::DoubleFernTop => ("double_plant_fern_top", WHITE, BiomeTint::Grass),
        Tile::SunflowerBottom => ("double_plant_sunflower_bottom", WHITE, BiomeTint::None),
        Tile::SunflowerTop => ("double_plant_sunflower_top", WHITE, BiomeTint::None),
        Tile::SunflowerFront => ("double_plant_sunflower_front", WHITE, BiomeTint::None),
        Tile::SunflowerBack => ("double_plant_sunflower_back", WHITE, BiomeTint::None),
        Tile::SyringaBottom => ("double_plant_syringa_bottom", WHITE, BiomeTint::None),
        Tile::SyringaTop => ("double_plant_syringa_top", WHITE, BiomeTint::None),
        Tile::RoseBushBottom => ("double_plant_rose_bottom", WHITE, BiomeTint::None),
        Tile::RoseBushTop => ("double_plant_rose_top", WHITE, BiomeTint::None),
        Tile::PeonyBottom => ("double_plant_paeonia_bottom", WHITE, BiomeTint::None),
        Tile::PeonyTop => ("double_plant_paeonia_top", WHITE, BiomeTint::None),
        Tile::Allium => ("flower_allium", WHITE, BiomeTint::None),
        Tile::BlueOrchid => ("flower_blue_orchid", WHITE, BiomeTint::None),
        Tile::Houstonia => ("flower_houstonia", WHITE, BiomeTint::None),
        Tile::OxeyeDaisy => ("flower_oxeye_daisy", WHITE, BiomeTint::None),
        Tile::TulipRed => ("flower_tulip_red", WHITE, BiomeTint::None),
        Tile::TulipOrange => ("flower_tulip_orange", WHITE, BiomeTint::None),
        Tile::TulipWhite => ("flower_tulip_white", WHITE, BiomeTint::None),
        Tile::TulipPink => ("flower_tulip_pink", WHITE, BiomeTint::None),
        Tile::PumpkinSide => ("pumpkin_side", WHITE, BiomeTint::None),
        Tile::PumpkinTop => ("pumpkin_top", WHITE, BiomeTint::None),
        Tile::PumpkinFace => ("pumpkin_face_off", WHITE, BiomeTint::None),
        Tile::StoneGranite => ("stone_granite", WHITE, BiomeTint::None),
        Tile::StoneDiorite => ("stone_diorite", WHITE, BiomeTint::None),
        Tile::StoneAndesite => ("stone_andesite", WHITE, BiomeTint::None),
        Tile::CoalOre => ("coal_ore", WHITE, BiomeTint::None),
        Tile::IronOre => ("iron_ore", WHITE, BiomeTint::None),
        Tile::GoldOre => ("gold_ore", WHITE, BiomeTint::None),
        Tile::RedstoneOre => ("redstone_ore", WHITE, BiomeTint::None),
        Tile::DiamondOre => ("diamond_ore", WHITE, BiomeTint::None),
        Tile::LapisOre => ("lapis_ore", WHITE, BiomeTint::None),
        Tile::EmeraldOre => ("emerald_ore", WHITE, BiomeTint::None),
        Tile::CocoaStage0 => ("cocoa_stage_0", WHITE, BiomeTint::None),
        Tile::CocoaStage1 => ("cocoa_stage_1", WHITE, BiomeTint::None),
        Tile::CocoaStage2 => ("cocoa_stage_2", WHITE, BiomeTint::None),
        Tile::BoneBlockSide => ("bone_block_side", WHITE, BiomeTint::None),
        Tile::BoneBlockTop => ("bone_block_top", WHITE, BiomeTint::None),
        Tile::PlanksOak => ("planks_oak", WHITE, BiomeTint::None),
        Tile::PlanksSpruce => ("planks_spruce", WHITE, BiomeTint::None),
        Tile::PlanksBirch => ("planks_birch", WHITE, BiomeTint::None),
        Tile::PlanksJungle => ("planks_jungle", WHITE, BiomeTint::None),
        Tile::PlanksAcacia => ("planks_acacia", WHITE, BiomeTint::None),
        Tile::PlanksDarkOak => ("planks_big_oak", WHITE, BiomeTint::None),
        Tile::StoneBrick => ("stonebrick", WHITE, BiomeTint::None),
        Tile::StoneBrickMossy => ("stonebrick_mossy", WHITE, BiomeTint::None),
        Tile::StoneBrickCracked => ("stonebrick_cracked", WHITE, BiomeTint::None),
        Tile::StoneBrickCarved => ("stonebrick_carved", WHITE, BiomeTint::None),
        Tile::SandstoneCarved => ("sandstone_carved", WHITE, BiomeTint::None),
        Tile::SandstoneSmooth => ("sandstone_smooth", WHITE, BiomeTint::None),
        Tile::RedSandstoneCarved => ("red_sandstone_carved", WHITE, BiomeTint::None),
        Tile::RedSandstoneSmooth => ("red_sandstone_smooth", WHITE, BiomeTint::None),
        Tile::Brick => ("brick", WHITE, BiomeTint::None),
        Tile::Glass => ("glass", WHITE, BiomeTint::None),
        Tile::Bookshelf => ("bookshelf", WHITE, BiomeTint::None),
        Tile::CraftingTableTop => ("crafting_table_top", WHITE, BiomeTint::None),
        Tile::CraftingTableSide => ("crafting_table_side", WHITE, BiomeTint::None),
        Tile::CraftingTableFront => ("crafting_table_front", WHITE, BiomeTint::None),
        Tile::StoneSlabTop => ("stone_slab_top", WHITE, BiomeTint::None),
        Tile::StoneSlabSide => ("stone_slab_side", WHITE, BiomeTint::None),
        Tile::WoolWhite => ("wool_colored_white", WHITE, BiomeTint::None),
        Tile::WoolOrange => ("wool_colored_orange", WHITE, BiomeTint::None),
        Tile::WoolMagenta => ("wool_colored_magenta", WHITE, BiomeTint::None),
        Tile::WoolLightBlue => ("wool_colored_light_blue", WHITE, BiomeTint::None),
        Tile::WoolYellow => ("wool_colored_yellow", WHITE, BiomeTint::None),
        Tile::WoolLime => ("wool_colored_lime", WHITE, BiomeTint::None),
        Tile::WoolPink => ("wool_colored_pink", WHITE, BiomeTint::None),
        Tile::WoolGray => ("wool_colored_gray", WHITE, BiomeTint::None),
        Tile::WoolSilver => ("wool_colored_silver", WHITE, BiomeTint::None),
        Tile::WoolCyan => ("wool_colored_cyan", WHITE, BiomeTint::None),
        Tile::WoolPurple => ("wool_colored_purple", WHITE, BiomeTint::None),
        Tile::WoolBlue => ("wool_colored_blue", WHITE, BiomeTint::None),
        Tile::WoolBrown => ("wool_colored_brown", WHITE, BiomeTint::None),
        Tile::WoolGreen => ("wool_colored_green", WHITE, BiomeTint::None),
        Tile::WoolRed => ("wool_colored_red", WHITE, BiomeTint::None),
        Tile::WoolBlack => ("wool_colored_black", WHITE, BiomeTint::None),
    }
}

struct Animation {
    sequence: Vec<usize>,
    ticks: u32,
    interpolate: bool,
}

struct Found {
    image: DecodedImage,
    frames: usize,
    animation: Option<Animation>,
}

pub fn load(zip_bytes: &[u8]) -> Result<TexturePack, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
        .map_err(|e| format!("not a zip archive: {e}"))?;
    let root = archive
        .file_names()
        .filter_map(Result::ok)
        .find_map(|name| name.find(BLOCKS_DIR).map(|at| name[..at].to_string()))
        .ok_or("the archive has no assets/minecraft/textures/blocks")?;
    let mut read = |path: &str| -> Option<Vec<u8>> {
        let file = archive.by_name(&format!("{root}{path}")).ok()?;
        let mut out = Vec::new();
        file.take(MAX_ENTRY_BYTES).read_to_end(&mut out).ok()?;
        Some(out)
    };
    let textures = build(&mut read, Some(&builtin().textures))?;
    let (description, icon) = read_meta(&mut read);
    let gui = gui_images(&mut read);
    let entities = entity_images(&mut read);
    Ok(TexturePack {
        textures,
        description,
        icon,
        gui,
        entities,
    })
}

pub fn builtin() -> TexturePack {
    let textures = build(&mut |path| builtin_file(path).map(<[u8]>::to_vec), None)
        .expect("built-in textures are complete");
    TexturePack {
        textures,
        description: "Faithful 32x".into(),
        icon: None,
        gui: builtin_gui(),
        entities: builtin_entities(),
    }
}

pub fn builtin_entities() -> EntityImages {
    entity_images(&mut |_| None)
}

fn entity_images(read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> EntityImages {
    let mut image = |path: &str, valid: fn(&DecodedImage) -> bool| {
        read(path)
            .and_then(|bytes| decode_png(&bytes).ok())
            .filter(valid)
            .unwrap_or_else(|| {
                let bytes =
                    builtin_entity_file(path).expect("built-in entity textures are complete");
                decode_png(bytes).expect("built-in entity textures are valid")
            })
    };
    let sheet = |i: &DecodedImage| i.width >= 64 && i.width == 2 * i.height;
    let pig = image(ENTITY_PIG, sheet);
    let chicken = image(ENTITY_CHICKEN, sheet);
    let cow = image(ENTITY_COW, sheet);
    let sheep = image(ENTITY_SHEEP, sheet);
    let sheep_fur = image(ENTITY_SHEEP_FUR, sheet);
    let mooshroom = image(ENTITY_MOOSHROOM, sheet);
    let polar_bear = image(ENTITY_POLAR_BEAR, sheet);
    let wolf = image(ENTITY_WOLF, sheet);
    let square = |i: &DecodedImage| i.width >= 128 && i.width == i.height;
    let horse = ENTITY_HORSE.map(|path| image(path, square));
    let horse_markings = ENTITY_HORSE_MARKINGS.map(|path| image(path, square));
    let donkey = image(ENTITY_DONKEY, square);
    let llama = ENTITY_LLAMA.map(|path| image(path, |i| i.width >= 128 && i.width == 2 * i.height));
    let rabbit = ENTITY_RABBIT.map(|path| image(path, sheet));
    let ocelot = image(ENTITY_OCELOT, sheet);
    let parrot = ENTITY_PARROT.map(|path| image(path, |i| i.width >= 64 && i.width == i.height));
    let squid = image(ENTITY_SQUID, sheet);
    let mushroom_red = image(BLOCK_MUSHROOM_RED, |i| {
        i.width >= MIN_TILE_SIZE && i.width == i.height
    });
    let spawn_egg = image(ITEM_SPAWN_EGG, |i| {
        i.width >= MIN_TILE_SIZE && i.width == i.height
    });
    let spawn_egg_overlay = image(ITEM_SPAWN_EGG_OVERLAY, |i| {
        i.width >= MIN_TILE_SIZE && i.width == i.height
    });
    EntityImages {
        pig,
        chicken,
        cow,
        sheep,
        sheep_fur,
        mooshroom,
        polar_bear,
        wolf,
        horse,
        horse_markings,
        donkey,
        llama,
        rabbit,
        ocelot,
        parrot,
        squid,
        mushroom_red,
        spawn_egg,
        spawn_egg_overlay,
    }
}

pub fn builtin_gui() -> GuiImages {
    gui_images(&mut |_| None)
}

fn gui_images(read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> GuiImages {
    let mut sheet = |name: &str| {
        read(&format!("{GUI_DIR}{name}"))
            .and_then(|bytes| {
                let image = decode_png(&bytes).ok()?;
                (image.width >= MIN_TILE_SIZE && image.width == image.height)
                    .then_some((bytes, image))
            })
            .unwrap_or_else(|| {
                let bytes = builtin_gui_file(name).expect("built-in gui sheets are complete");
                let image = decode_png(bytes).expect("built-in gui sheets are valid");
                (bytes.to_vec(), image)
            })
    };
    let (widgets, _) = sheet(GUI_WIDGETS);
    let (icons, _) = sheet(GUI_ICONS);
    let (inventory, image) = sheet(GUI_INVENTORY);
    let (panel, panel_slice) = panel_image(&image);
    GuiImages {
        widgets,
        icons,
        inventory,
        panel,
        panel_slice,
    }
}

fn panel_image(sheet: &DecodedImage) -> (Vec<u8>, u32) {
    let width = sheet.width;
    let unit = |v: u32| v * width / GUI_SIZE_UNITS;
    let corner = unit(PANEL_CORNER_UNITS).max(1);
    let right = unit(PANEL_WIDTH_UNITS) - corner;
    let bottom = unit(PANEL_HEIGHT_UNITS) - corner;
    let edge = unit(PANEL_EDGE_UNITS);
    let out = corner * 3;
    let mut rgba = vec![0u8; (out * out * 4) as usize];
    let cells = [
        ((0, 0), (0, 0)),
        ((1, 0), (edge, 0)),
        ((2, 0), (right, 0)),
        ((0, 1), (0, edge)),
        ((1, 1), (unit(PANEL_FLAT_X_UNITS), unit(PANEL_FLAT_Y_UNITS))),
        ((2, 1), (right, edge)),
        ((0, 2), (0, bottom)),
        ((1, 2), (edge, bottom)),
        ((2, 2), (right, bottom)),
    ];
    for ((cx, cy), (sx, sy)) in cells {
        for y in 0..corner {
            for x in 0..corner {
                let from = (((sy + y) * width + sx + x) * 4) as usize;
                let to = (((cy * corner + y) * out + cx * corner + x) * 4) as usize;
                rgba[to..to + 4].copy_from_slice(&sheet.rgba[from..from + 4]);
            }
        }
    }
    (encode_png(out, &rgba), corner)
}

fn encode_png(size: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(rgba).expect("png data");
    writer.finish().expect("png end");
    out
}

macro_rules! builtin_files {
    ($($file:literal),* $(,)?) => {
        &[$(($file, include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/assets/textures/", $file
        )))),*]
    };
}

const BUILTIN: &[(&str, &[u8])] = builtin_files![
    "grass_top.png",
    "grass_side.png",
    "grass_side_overlay.png",
    "dirt.png",
    "stone.png",
    "sand.png",
    "log_oak.png",
    "log_oak_top.png",
    "leaves_oak.png",
    "water_still.png",
    "water_still.png.mcmeta",
    "glowstone.png",
    "tallgrass.png",
    "flower_rose.png",
    "flower_dandelion.png",
    "torch_on.png",
    "gravel.png",
    "lava_still.png",
    "lava_still.png.mcmeta",
    "obsidian.png",
    "cobblestone.png",
    "log_birch.png",
    "log_birch_top.png",
    "leaves_birch.png",
    "log_spruce.png",
    "log_spruce_top.png",
    "leaves_spruce.png",
    "bedrock.png",
    "grass_side_snowed.png",
    "snow.png",
    "ice.png",
    "cactus_top.png",
    "cactus_side.png",
    "cactus_bottom.png",
    "deadbush.png",
    "red_sand.png",
    "hardened_clay.png",
    "hardened_clay_stained_white.png",
    "hardened_clay_stained_orange.png",
    "hardened_clay_stained_yellow.png",
    "hardened_clay_stained_brown.png",
    "hardened_clay_stained_red.png",
    "hardened_clay_stained_silver.png",
    "reeds.png",
    "sandstone_top.png",
    "sandstone_normal.png",
    "sandstone_bottom.png",
    "red_sandstone_top.png",
    "red_sandstone_normal.png",
    "red_sandstone_bottom.png",
    "log_jungle.png",
    "log_jungle_top.png",
    "leaves_jungle.png",
    "log_acacia.png",
    "log_acacia_top.png",
    "leaves_acacia.png",
    "log_big_oak.png",
    "log_big_oak_top.png",
    "leaves_big_oak.png",
    "vine.png",
    "melon_side.png",
    "melon_top.png",
    "pumpkin_face_off.png",
    "pumpkin_side.png",
    "pumpkin_top.png",
    "stone_granite.png",
    "stone_diorite.png",
    "stone_andesite.png",
    "coal_ore.png",
    "iron_ore.png",
    "gold_ore.png",
    "redstone_ore.png",
    "diamond_ore.png",
    "lapis_ore.png",
    "emerald_ore.png",
    "cocoa_stage_0.png",
    "cocoa_stage_1.png",
    "cocoa_stage_2.png",
    "bone_block_side.png",
    "bone_block_top.png",
    "mushroom_block_skin_brown.png",
    "mushroom_block_skin_red.png",
    "mushroom_block_skin_stem.png",
    "mushroom_block_inside.png",
    "waterlily.png",
    "mycelium_top.png",
    "mycelium_side.png",
    "mushroom_brown.png",
    "mushroom_red.png",
    "clay.png",
    "ice_packed.png",
    "dirt_podzol_top.png",
    "dirt_podzol_side.png",
    "coarse_dirt.png",
    "cobblestone_mossy.png",
    "fern.png",
    "double_plant_grass_bottom.png",
    "double_plant_grass_top.png",
    "double_plant_fern_bottom.png",
    "double_plant_fern_top.png",
    "double_plant_sunflower_bottom.png",
    "double_plant_sunflower_top.png",
    "double_plant_sunflower_front.png",
    "double_plant_sunflower_back.png",
    "double_plant_syringa_bottom.png",
    "double_plant_syringa_top.png",
    "double_plant_rose_bottom.png",
    "double_plant_rose_top.png",
    "double_plant_paeonia_bottom.png",
    "double_plant_paeonia_top.png",
    "flower_allium.png",
    "flower_blue_orchid.png",
    "flower_houstonia.png",
    "flower_oxeye_daisy.png",
    "flower_tulip_red.png",
    "flower_tulip_orange.png",
    "flower_tulip_white.png",
    "flower_tulip_pink.png",
    "planks_oak.png",
    "planks_spruce.png",
    "planks_birch.png",
    "planks_jungle.png",
    "planks_acacia.png",
    "planks_big_oak.png",
    "stonebrick.png",
    "stonebrick_mossy.png",
    "stonebrick_cracked.png",
    "stonebrick_carved.png",
    "sandstone_carved.png",
    "sandstone_smooth.png",
    "red_sandstone_carved.png",
    "red_sandstone_smooth.png",
    "brick.png",
    "glass.png",
    "bookshelf.png",
    "crafting_table_top.png",
    "crafting_table_side.png",
    "crafting_table_front.png",
    "stone_slab_top.png",
    "stone_slab_side.png",
    "wool_colored_white.png",
    "wool_colored_orange.png",
    "wool_colored_magenta.png",
    "wool_colored_light_blue.png",
    "wool_colored_yellow.png",
    "wool_colored_lime.png",
    "wool_colored_pink.png",
    "wool_colored_gray.png",
    "wool_colored_silver.png",
    "wool_colored_cyan.png",
    "wool_colored_purple.png",
    "wool_colored_blue.png",
    "wool_colored_brown.png",
    "wool_colored_green.png",
    "wool_colored_red.png",
    "wool_colored_black.png",
];

fn builtin_file(path: &str) -> Option<&'static [u8]> {
    let name = path.strip_prefix(BLOCKS_DIR)?;
    BUILTIN
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, bytes)| *bytes)
}

const BUILTIN_GUI: &[(&str, &[u8])] = builtin_files![
    "gui/widgets.png",
    "gui/icons.png",
    "gui/container/inventory.png",
];

fn builtin_gui_file(name: &str) -> Option<&'static [u8]> {
    BUILTIN_GUI
        .iter()
        .find(|(file, _)| file.strip_prefix("gui/") == Some(name))
        .map(|(_, bytes)| *bytes)
}

const BUILTIN_ENTITIES: &[(&str, &[u8])] = &[
    (
        ENTITY_HORSE[0],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_white.png"
        )),
    ),
    (
        ENTITY_HORSE[1],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_creamy.png"
        )),
    ),
    (
        ENTITY_HORSE[2],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_chestnut.png"
        )),
    ),
    (
        ENTITY_HORSE[3],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_brown.png"
        )),
    ),
    (
        ENTITY_HORSE[4],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_black.png"
        )),
    ),
    (
        ENTITY_HORSE[5],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_gray.png"
        )),
    ),
    (
        ENTITY_HORSE[6],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_darkbrown.png"
        )),
    ),
    (
        ENTITY_HORSE_MARKINGS[0],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_markings_white.png"
        )),
    ),
    (
        ENTITY_HORSE_MARKINGS[1],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_markings_whitefield.png"
        )),
    ),
    (
        ENTITY_HORSE_MARKINGS[2],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_markings_whitedots.png"
        )),
    ),
    (
        ENTITY_HORSE_MARKINGS[3],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/horse_markings_blackdots.png"
        )),
    ),
    (
        ENTITY_DONKEY,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/horse/donkey.png"
        )),
    ),
    (
        ENTITY_LLAMA[0],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/llama/llama_creamy.png"
        )),
    ),
    (
        ENTITY_LLAMA[1],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/llama/llama_white.png"
        )),
    ),
    (
        ENTITY_LLAMA[2],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/llama/llama_brown.png"
        )),
    ),
    (
        ENTITY_LLAMA[3],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/llama/llama_gray.png"
        )),
    ),
    (
        ENTITY_RABBIT[0],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/brown.png"
        )),
    ),
    (
        ENTITY_RABBIT[1],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/white.png"
        )),
    ),
    (
        ENTITY_RABBIT[2],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/black.png"
        )),
    ),
    (
        ENTITY_RABBIT[3],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/white_splotched.png"
        )),
    ),
    (
        ENTITY_RABBIT[4],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/gold.png"
        )),
    ),
    (
        ENTITY_RABBIT[5],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/rabbit/salt.png"
        )),
    ),
    (
        ENTITY_OCELOT,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/cat/ocelot.png"
        )),
    ),
    (
        ENTITY_PARROT[0],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/parrot/parrot_red_blue.png"
        )),
    ),
    (
        ENTITY_PARROT[1],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/parrot/parrot_blue.png"
        )),
    ),
    (
        ENTITY_PARROT[2],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/parrot/parrot_green.png"
        )),
    ),
    (
        ENTITY_PARROT[3],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/parrot/parrot_yellow_blue.png"
        )),
    ),
    (
        ENTITY_PARROT[4],
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/parrot/parrot_grey.png"
        )),
    ),
    (
        ENTITY_SQUID,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/squid.png"
        )),
    ),
    (
        ENTITY_PIG,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/pig/pig.png"
        )),
    ),
    (
        ENTITY_CHICKEN,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/chicken.png"
        )),
    ),
    (
        ENTITY_COW,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/cow/cow.png"
        )),
    ),
    (
        ENTITY_SHEEP,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/sheep/sheep.png"
        )),
    ),
    (
        ENTITY_SHEEP_FUR,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/sheep/sheep_fur.png"
        )),
    ),
    (
        ENTITY_MOOSHROOM,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/cow/mooshroom.png"
        )),
    ),
    (
        ENTITY_POLAR_BEAR,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/bear/polarbear.png"
        )),
    ),
    (
        ENTITY_WOLF,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/entity/wolf/wolf.png"
        )),
    ),
    (
        BLOCK_MUSHROOM_RED,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/mushroom_red.png"
        )),
    ),
    (
        ITEM_SPAWN_EGG,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/items/spawn_egg.png"
        )),
    ),
    (
        ITEM_SPAWN_EGG_OVERLAY,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/items/spawn_egg_overlay.png"
        )),
    ),
];

fn builtin_entity_file(path: &str) -> Option<&'static [u8]> {
    BUILTIN_ENTITIES
        .iter()
        .find(|(file, _)| *file == path)
        .map(|(_, bytes)| *bytes)
}

fn build(
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    fallback: Option<&TextureSet>,
) -> Result<TextureSet, String> {
    let mut found: Vec<Option<Found>> = Vec::new();
    for &tile in &Tile::ALL {
        let stem = source(tile).0;
        let image = read(&format!("{BLOCKS_DIR}{stem}.png")).and_then(|b| decode_png(&b).ok());
        let meta = read(&format!("{BLOCKS_DIR}{stem}.png.mcmeta"));
        let tile_found = image.and_then(|image| {
            if image.width == 0 || image.height < image.width {
                return None;
            }
            let frames = (image.height / image.width) as usize;
            let animation = meta.and_then(|m| parse_animation(&m, frames));
            Some(Found {
                image,
                frames,
                animation,
            })
        });
        if tile_found.is_none() && fallback.is_none() {
            return Err(format!("missing texture {stem}.png"));
        }
        found.push(tile_found);
    }
    if found.iter().all(Option::is_none) {
        return Err("the archive has no usable block textures".into());
    }

    let widest = found
        .iter()
        .flatten()
        .map(|f| f.image.width)
        .max()
        .unwrap_or(0);
    let size = widest.next_power_of_two().max(MIN_TILE_SIZE);
    if size > MAX_TILE_SIZE {
        return Err(format!("textures are too large: {widest}px"));
    }

    let overlay = found[Tile::GrassSide.layer() as usize]
        .as_ref()
        .and_then(|_| read(&format!("{BLOCKS_DIR}grass_side_overlay.png")))
        .and_then(|b| decode_png(&b).ok())
        .filter(|image| image.width > 0 && image.height >= image.width);

    let mut tiles = Vec::new();
    for (&tile, found) in Tile::ALL.iter().zip(&found) {
        let Some(found) = found else {
            let fallback = fallback.expect("checked while reading");
            tiles.push(scaled(fallback, tile, size));
            continue;
        };
        let mut texture = build_tile(found, size);
        let (_, tint, biome) = source(tile);
        texture.tint = tint;
        texture.biome = biome;
        if tile == Tile::GrassSide
            && let Some(overlay) = &overlay
        {
            add_overlay(&mut texture, overlay, size);
        }
        if tile == Tile::Water {
            for p in texture.layers.as_chunks_mut::<4>().0 {
                p[3] = p[3].min(WATER_MAX_ALPHA);
            }
        }
        tiles.push(texture);
    }

    let layers: u32 = tiles.iter().map(TileTexture::layer_span).sum();
    if layers as usize > MAX_LAYERS {
        return Err(format!(
            "too many animation frames: {layers} (limit {MAX_LAYERS})"
        ));
    }
    let mut colormap = |name: &str| {
        read(&format!("{COLORMAP_DIR}{name}.png"))
            .and_then(|b| decode_png(&b).ok())
            .and_then(|image| Colormap::from_image(&image))
    };
    let palette =
        BiomePalette::from_colormaps(colormap("grass").as_ref(), colormap("foliage").as_ref());
    Ok(TextureSet::from_tiles(size, tiles, palette))
}

fn build_tile(found: &Found, size: u32) -> TileTexture {
    let width = found.image.width;
    let frame_bytes = (width * width * 4) as usize;
    let frame = |index: usize| {
        scale_nearest(
            &found.image.rgba[index * frame_bytes..(index + 1) * frame_bytes],
            width,
            size,
        )
    };
    let (sequence, ticks, interpolate) = match &found.animation {
        Some(a) => (a.sequence.clone(), a.ticks, a.interpolate),
        None => ((0..found.frames).collect(), 1, false),
    };
    if sequence.len() == 1 {
        return TileTexture::still(frame(sequence[0]));
    }
    animated(&frame, &sequence, ticks, interpolate)
}

fn animated(
    frame: &impl Fn(usize) -> Vec<u8>,
    sequence: &[usize],
    ticks: u32,
    interpolate: bool,
) -> TileTexture {
    TileTexture {
        layers: sequence.iter().flat_map(|&i| frame(i)).collect(),
        frames: sequence.len() as u32,
        fps: 20.0 / ticks as f32,
        tint: WHITE,
        interpolate,
        biome: BiomeTint::None,
        overlay: false,
    }
}

fn scaled(standard: &TextureSet, tile: Tile, size: u32) -> TileTexture {
    let src = standard.tile(tile);
    let from = standard.size();
    let bytes = standard.frame_bytes();
    TileTexture {
        layers: src
            .layers
            .chunks_exact(bytes)
            .flat_map(|f| scale_nearest(f, from, size))
            .collect(),
        frames: src.frames,
        fps: src.fps,
        tint: src.tint,
        interpolate: src.interpolate,
        biome: src.biome,
        overlay: src.overlay,
    }
}

fn add_overlay(texture: &mut TileTexture, overlay: &DecodedImage, size: u32) {
    let width = overlay.width;
    let top = scale_nearest(&overlay.rgba[..(width * width * 4) as usize], width, size);
    texture.layers.extend_from_slice(&top);
    texture.overlay = true;
}

fn parse_animation(mcmeta: &[u8], frames: usize) -> Option<Animation> {
    let json: Value = serde_json::from_slice(mcmeta).ok()?;
    let anim = json.get("animation")?;
    let default_time = anim
        .get("frametime")
        .and_then(Value::as_u64)
        .map_or(1, |t| t.clamp(1, 1 << 16) as u32);
    let interpolate = anim
        .get("interpolate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let entries: Vec<(usize, u32)> = match anim.get("frames").and_then(Value::as_array) {
        Some(list) => list
            .iter()
            .filter_map(|item| match item {
                Value::Number(n) => Some((n.as_u64()? as usize, default_time)),
                Value::Object(o) => {
                    let time = o
                        .get("time")
                        .and_then(Value::as_u64)
                        .map_or(default_time, |t| t.clamp(1, 1 << 16) as u32);
                    Some((o.get("index")?.as_u64()? as usize, time))
                }
                _ => None,
            })
            .filter(|&(index, _)| index < frames)
            .collect(),
        None => (0..frames).map(|i| (i, default_time)).collect(),
    };
    if entries.is_empty() {
        return None;
    }
    let ticks = entries.iter().fold(0, |acc, &(_, t)| gcd(acc, t));
    let sequence = entries
        .iter()
        .flat_map(|&(index, time)| std::iter::repeat_n(index, (time / ticks) as usize))
        .collect();
    Some(Animation {
        sequence,
        ticks,
        interpolate,
    })
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn read_meta(read: &mut impl FnMut(&str) -> Option<Vec<u8>>) -> (String, Option<Vec<u8>>) {
    let description = read("pack.mcmeta")
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|json| match json.get("pack")?.get("description")? {
            Value::String(s) => Some(strip_formatting(s)),
            Value::Object(o) => Some(strip_formatting(o.get("text")?.as_str()?)),
            _ => None,
        })
        .unwrap_or_default();
    (description, read("pack.png"))
}

pub fn strip_formatting(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests;

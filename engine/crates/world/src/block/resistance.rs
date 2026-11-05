use super::{BASE_BLOCKS, Block, BlockDef};

pub(super) const fn with_blast_resistance(mut blocks: [BlockDef; 186]) -> [BlockDef; 186] {
    let mut i = 0;
    while i < blocks.len() {
        blocks[i].blast_resistance = blast_resistance(Block::ALL[i]);
        i += 1;
    }
    blocks
}

pub(super) const BLOCKS_WITH_RESISTANCE: [BlockDef; 186] = with_blast_resistance(BASE_BLOCKS);

/// Значение `Block.getExplosionResistance` 1.12 (внутреннее сопротивление, делённое на 5).
const fn blast_resistance(block: Block) -> f32 {
    use Block::*;
    match block {
        Air | TallGrass | Poppy | Dandelion | Torch | DeadBush | Reeds | Fern
        | DoubleGrassLower | DoubleGrassUpper | DoubleFernLower | DoubleFernUpper
        | SunflowerLower | SunflowerUpper | LilacLower | LilacUpper | RoseBushLower
        | RoseBushUpper | PeonyLower | PeonyUpper | Allium | BlueOrchid | Houstonia
        | OxeyeDaisy | RedTulip | OrangeTulip | WhiteTulip | PinkTulip | BrownMushroom
        | RedMushroom | Waterlily | FlowerPot | RedstoneTorch | Tnt | TripwireHook | Tripwire => {
            0.0
        }
        Snow | Carpet => 0.1,
        Leaves | BirchLeaves | SpruceLeaves | JungleLeaves | AcaciaLeaves | DarkOakLeaves
        | Vine | SnowBlock | BrownMushroomBlock | RedMushroomBlock | MushroomStem | Bed => 0.2,
        Glowstone | Glass | GlassPane => 0.3,
        Cactus | Ladder => 0.4,
        Dirt | Sand | RedSand | Podzol | CoarseDirt | Ice | IcePacked | BrewingStand
        | StonePressurePlate | Lever => 0.5,
        Grass | SnowyGrass | Gravel | Clay | Mycelium => 0.6,
        Rail => 0.7,
        MonsterEgg => 0.75,
        Wool | Sandstone | RedSandstone | ChiseledSandstone | SmoothSandstone
        | ChiseledRedSandstone | SmoothRedSandstone | SandstoneStairs | RedSandstoneStairs => 0.8,
        Melon | Pumpkin => 1.0,
        Bookshelf => 1.5,
        Log | BirchLog | SpruceLog | JungleLog | AcaciaLog | DarkOakLog | BoneBlock | Cauldron => {
            2.0
        }
        CraftingTable | Chest => 2.5,
        CoalOre | IronOre | GoldOre | RedstoneOre | DiamondOre | LapisOre | EmeraldOre | Cocoa
        | OakPlanks | SprucePlanks | BirchPlanks | JunglePlanks | AcaciaPlanks | DarkOakPlanks
        | OakSlab | SpruceSlab | BirchSlab | JungleSlab | AcaciaSlab | DarkOakSlab | OakStairs
        | SpruceStairs | BirchStairs | JungleStairs | AcaciaStairs | DarkOakStairs | OakFence
        | SpruceFence | BirchFence | JungleFence | AcaciaFence | DarkOakFence | OakFenceGate
        | SpruceFenceGate | BirchFenceGate | JungleFenceGate | AcaciaFenceGate
        | DarkOakFenceGate | OakDoor | OakDoorUpper | SpruceDoor | SpruceDoorUpper | BirchDoor
        | BirchDoorUpper | JungleDoor | JungleDoorUpper | AcaciaDoor | AcaciaDoorUpper
        | DarkOakDoor | DarkOakDoorUpper | OakTrapdoor => 3.0,
        Furnace | LitFurnace | Dispenser => 3.5,
        Web => 4.0,
        Terracotta | WhiteTerracotta | OrangeTerracotta | YellowTerracotta | BrownTerracotta
        | RedTerracotta | SilverTerracotta | BlueTerracotta => 4.2,
        MobSpawner | IronDoor | IronDoorUpper => 5.0,
        Stone | Granite | Diorite | Andesite | Cobblestone | MossyCobblestone | StoneBricks
        | MossyStoneBricks | CrackedStoneBricks | ChiseledStoneBricks | Bricks | StoneSlab
        | SandstoneSlab | CobblestoneSlab | BrickSlab | StoneBrickSlab | RedSandstoneSlab
        | CobblestoneStairs | BrickStairs | StoneBrickStairs | IronBars => 6.0,
        Water | Lava => 100.0,
        Obsidian => 1200.0,
        Bedrock => 3_600_000.0,
        Barrier => 3_600_002.4,
    }
}

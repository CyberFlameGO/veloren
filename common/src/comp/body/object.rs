use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Body {
    Arrow = 0,
    Bomb = 1,
    Scarecrow = 2,
    Cauldron = 3,
    ChestVines = 4,
    Chest = 5,
    ChestDark = 6,
    ChestDemon = 7,
    ChestGold = 8,
    ChestLight = 9,
    ChestOpen = 10,
    ChestSkull = 11,
    Pumpkin = 12,
    Pumpkin2 = 13,
    Pumpkin3 = 14,
    Pumpkin4 = 15,
    Pumpkin5 = 16,
    Campfire = 17,
    LanternGround = 18,
    LanternGroundOpen = 19,
    LanternStanding2 = 20,
    LanternStanding = 21,
    PotionBlue = 22,
    PotionGreen = 23,
    PotionRed = 24,
    Crate = 25,
    Tent = 26,
    WindowSpooky = 27,
    DoorSpooky = 28,
    Anvil = 29,
    Gravestone = 30,
    Gravestone2 = 31,
    Bench = 32,
    Chair = 33,
    Chair2 = 34,
    Chair3 = 35,
    Table = 36,
    Table2 = 37,
    Table3 = 38,
    Drawer = 39,
    BedBlue = 40,
    Carpet = 41,
    Bedroll = 42,
    CarpetHumanRound = 43,
    CarpetHumanSquare = 44,
    CarpetHumanSquare2 = 45,
    CarpetHumanSquircle = 46,
    Pouch = 47,
    CraftingBench = 48,
    BoltFire = 49,
    ArrowSnake = 50,
    CampfireLit = 51,
    BoltFireBig = 52,
    TrainingDummy = 53,
    FireworkBlue = 54,
    FireworkGreen = 55,
    FireworkPurple = 56,
    FireworkRed = 57,
    FireworkYellow = 58,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        *(&ALL_OBJECTS).choose(&mut rng).unwrap()
    }
}

pub const ALL_OBJECTS: [Body; 59] = [
    Body::Arrow,
    Body::Bomb,
    Body::Scarecrow,
    Body::Cauldron,
    Body::ChestVines,
    Body::Chest,
    Body::ChestDark,
    Body::ChestDemon,
    Body::ChestGold,
    Body::ChestLight,
    Body::ChestOpen,
    Body::ChestSkull,
    Body::Pumpkin,
    Body::Pumpkin2,
    Body::Pumpkin3,
    Body::Pumpkin4,
    Body::Pumpkin5,
    Body::Campfire,
    Body::CampfireLit,
    Body::LanternGround,
    Body::LanternGroundOpen,
    Body::LanternStanding,
    Body::LanternStanding2,
    Body::PotionRed,
    Body::PotionBlue,
    Body::PotionGreen,
    Body::Crate,
    Body::Tent,
    Body::WindowSpooky,
    Body::DoorSpooky,
    Body::Anvil,
    Body::Gravestone,
    Body::Gravestone2,
    Body::Bench,
    Body::Chair,
    Body::Chair2,
    Body::Chair3,
    Body::Table,
    Body::Table2,
    Body::Table3,
    Body::Drawer,
    Body::BedBlue,
    Body::Carpet,
    Body::Bedroll,
    Body::CarpetHumanRound,
    Body::CarpetHumanSquare,
    Body::CarpetHumanSquare2,
    Body::CarpetHumanSquircle,
    Body::Pouch,
    Body::CraftingBench,
    Body::BoltFire,
    Body::BoltFireBig,
    Body::ArrowSnake,
    Body::TrainingDummy,
    Body::FireworkBlue,
    Body::FireworkGreen,
    Body::FireworkPurple,
    Body::FireworkRed,
    Body::FireworkYellow,
];

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Object(body) }
}

impl Body {
    pub fn to_string(&self) -> &str {
        match self {
            Body::Arrow => "arrow",
            Body::Bomb => "bomb",
            Body::Scarecrow => "scarecrow",
            Body::Cauldron => "cauldron",
            Body::ChestVines => "chest_vines",
            Body::Chest => "chest",
            Body::ChestDark => "chest_dark",
            Body::ChestDemon => "chest_demon",
            Body::ChestGold => "chest_gold",
            Body::ChestLight => "chest_light",
            Body::ChestOpen => "chest_open",
            Body::ChestSkull => "chest_skull",
            Body::Pumpkin => "pumpkin",
            Body::Pumpkin2 => "pumpkin_2",
            Body::Pumpkin3 => "pumpkin_3",
            Body::Pumpkin4 => "pumpkin_4",
            Body::Pumpkin5 => "pumpkin_5",
            Body::Campfire => "campfire",
            Body::CampfireLit => "campfire_lit",
            Body::LanternGround => "lantern_ground",
            Body::LanternGroundOpen => "lantern_ground_open",
            Body::LanternStanding => "lantern_standing",
            Body::LanternStanding2 => "lantern_standing_2",
            Body::PotionRed => "potion_red",
            Body::PotionBlue => "potion_blue",
            Body::PotionGreen => "potion_green",
            Body::Crate => "crate",
            Body::Tent => "tent",
            Body::WindowSpooky => "window_spooky",
            Body::DoorSpooky => "door_spooky",
            Body::Anvil => "anvil",
            Body::Gravestone => "gravestone",
            Body::Gravestone2 => "gravestone_2",
            Body::Bench => "bench",
            Body::Chair => "chair",
            Body::Chair2 => "chair_2",
            Body::Chair3 => "chair_3",
            Body::Table => "table",
            Body::Table2 => "table_2",
            Body::Table3 => "table_3",
            Body::Drawer => "drawer",
            Body::BedBlue => "bed_blue",
            Body::Carpet => "carpet",
            Body::Bedroll => "bedroll",
            Body::CarpetHumanRound => "carpet_human_round",
            Body::CarpetHumanSquare => "carpet_human_square",
            Body::CarpetHumanSquare2 => "carpet_human_square_2",
            Body::CarpetHumanSquircle => "carpet_human_squircle",
            Body::Pouch => "pouch",
            Body::CraftingBench => "crafting_bench",
            Body::BoltFire => "bolt_fire",
            Body::BoltFireBig => "bolt_fire_big",
            Body::ArrowSnake => "arrow_snake",
            Body::TrainingDummy => "training_dummy",
            Body::FireworkBlue => "firework_blue",
            Body::FireworkGreen => "firework_green",
            Body::FireworkPurple => "firework_purple",
            Body::FireworkRed => "firework_red",
            Body::FireworkYellow => "firework_yellow",
        }
    }
}

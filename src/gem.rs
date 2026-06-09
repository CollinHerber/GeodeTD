use bevy::prelude::*;

// Each gem's data and on-hit behavior lives in its own file under `gem/` so the
// kinds stay separated by concern. `GemKind` below is the shared handle the rest
// of the game uses; it just dispatches to the per-gem `def()`.
mod amethyst;
mod aquamarine;
mod diamond;
mod emerald;
mod opal;
mod ruby;
mod sapphire;
mod topaz;

const TOWER_RANGE_MULTIPLIER: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GemKind {
    Ruby,
    Sapphire,
    Topaz,
    Emerald,
    Amethyst,
    Diamond,
    Aquamarine,
    Opal,
}

/// The full static definition of a gem, assembled in its own module under `gem/`.
/// Private to this module — the outside world goes through `GemKind`'s methods.
struct GemDef {
    name: &'static str,
    srgb: [f32; 3],
    /// Base (Chipped) tower stats before any grade multiplier.
    chipped: TowerStats,
    /// The special on-hit behavior, scaled by the tower's grade.
    effect: fn(GemGrade) -> GemEffect,
}

impl GemKind {
    /// Single dispatch point: adding a gem means one arm here plus its file.
    fn def(self) -> GemDef {
        match self {
            GemKind::Ruby => ruby::def(),
            GemKind::Sapphire => sapphire::def(),
            GemKind::Topaz => topaz::def(),
            GemKind::Emerald => emerald::def(),
            GemKind::Amethyst => amethyst::def(),
            GemKind::Diamond => diamond::def(),
            GemKind::Aquamarine => aquamarine::def(),
            GemKind::Opal => opal::def(),
        }
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub fn srgb(self) -> [f32; 3] {
        self.def().srgb
    }

    pub fn chipped_stats(self) -> TowerStats {
        self.def().chipped
    }

    /// The special on-hit behavior for this gem, scaled by the tower's grade.
    pub fn effect(self, grade: GemGrade) -> GemEffect {
        (self.def().effect)(grade)
    }
}

/// Special behavior layered on top of a tower's base damage. Magnitudes already
/// account for grade where relevant (see each gem module's `effect`).
#[derive(Clone, Copy)]
pub enum GemEffect {
    None,
    /// Multiplies enemy speed by `factor` (< 1.0) for `duration` seconds.
    Slow {
        factor: f32,
        duration: f32,
    },
    /// Deals `damage_fraction` of the hit to other enemies within `radius`.
    Splash {
        radius: f32,
        damage_fraction: f32,
    },
    /// Splash damage that also slows the primary and splash targets.
    SlowSplash {
        radius: f32,
        damage_fraction: f32,
        slow_factor: f32,
        slow_duration: f32,
    },
    /// Splash damage plus a one-time support boost on upgrade, used by Star
    /// Yellow Sapphire.
    SlowSplashBoost {
        radius: f32,
        damage_fraction: f32,
        slow_factor: f32,
        slow_duration: f32,
        damage_bonus: f32,
        boost_range: f32,
    },
    /// Stacking damage-over-time; each hit adds a stack up to `max_stacks` and
    /// refreshes the `duration`. Also slows by `slow_factor` (a slowing poison).
    Poison {
        dps_per_stack: f32,
        duration: f32,
        max_stacks: u32,
        slow_factor: f32,
    },
    /// Strikes up to `targets` separate enemies in range, the extras taking
    /// `damage_fraction` of the hit.
    Multi {
        targets: u32,
        damage_fraction: f32,
    },
    /// Deals `damage_fraction` of the hit to every other enemy inside the
    /// tower's attack range.
    Area {
        damage_fraction: f32,
    },
    /// Deals `multiplier`x damage to its favored enemy class — air when `air` is
    /// true, ground otherwise — and normal damage to the rest.
    Favored {
        air: bool,
        multiplier: f32,
    },
    /// Support aura: reduces the cooldown of towers within range by
    /// `cooldown_reduction` (a fraction in `[0, 1]`). Does nothing on hit.
    Haste {
        cooldown_reduction: f32,
    },
    /// One-time support upgrade: boosts already-placed towers within `range` by
    /// `damage_bonus` when the special gem is upgraded. Does nothing on hit.
    DamageBoost {
        damage_bonus: f32,
        range: f32,
    },
    /// Ancient Blood Stone proc package: chance for a larger hit and chance to
    /// apply a burning damage-over-time effect.
    AncientBlood {
        triple_chance: f32,
        triple_multiplier: f32,
        burn_chance: f32,
        burn_dps: f32,
        burn_duration: f32,
    },
    /// Gold proc package: chance for a larger hit and a timed armor reduction
    /// approximation that makes later tower hits deal more damage.
    ArmorBreak {
        crit_chance: f32,
        crit_multiplier: f32,
        armor_reduction: f32,
        duration: f32,
    },
    /// Jade poison package: slowing poison, plus optional Lucky Asian Jade
    /// crit, stun, and bonus-gold rolls.
    JadePoison {
        dps: f32,
        duration: f32,
        slow_factor: f32,
        crit_chance: f32,
        crit_multiplier: f32,
        stun_chance: f32,
        stun_duration: f32,
        gold_chance: f32,
    },
    /// Single-target critical-hit package, optionally restricted to grounded
    /// enemies for specials like Pink Diamond.
    Critical {
        chance: f32,
        multiplier: f32,
        ground_only: bool,
    },
    /// Air-only single-target tower that reduces armor for flying enemies inside
    /// its range.
    AirArmorAura {
        armor_reduction: f32,
    },
    /// Tourmaline package: direct attack, timed damage proc, and ground armor
    /// reduction aura.
    Tourmaline {
        proc_chance: f32,
        proc_total_damage: f32,
        proc_duration: f32,
        armor_reduction: f32,
        armor_range: f32,
    },
    /// Persistent movement slow for enemies inside an aura while the tower still
    /// uses its normal direct attack.
    SlowAura {
        factor: f32,
        radius: f32,
        duration: f32,
        ignores_opals: bool,
    },
    /// Continuous damage to every enemy inside the tower's attack range.
    DamageAura {
        dps: f32,
    },
}

impl GemEffect {
    pub fn uses_direct_attack(self) -> bool {
        !matches!(self, GemEffect::DamageAura { .. })
    }

    pub fn ignores_opal_effects(self) -> bool {
        matches!(
            self,
            GemEffect::DamageAura { .. }
                | GemEffect::SlowAura {
                    ignores_opals: true,
                    ..
                }
        )
    }

    pub fn can_target(self, flying: bool) -> bool {
        match self {
            GemEffect::Critical {
                ground_only: true, ..
            } => !flying,
            GemEffect::AirArmorAura { .. } => flying,
            _ => true,
        }
    }

    /// Short one-line summary for the tower stat panel.
    pub fn describe(self) -> String {
        match self {
            GemEffect::None => "Effect: none".to_string(),
            GemEffect::Slow { factor, duration } => {
                format!("Slow: -{:.0}% for {:.1}s", (1.0 - factor) * 100.0, duration)
            }
            GemEffect::Splash {
                radius,
                damage_fraction,
            } => format!("Splash: {:.0}% in r{:.0}", damage_fraction * 100.0, radius),
            GemEffect::SlowSplash {
                radius,
                damage_fraction,
                slow_factor,
                slow_duration,
            } => format!(
                "Splash: {:.0}% r{:.0}; slow -{:.0}% for {:.1}s",
                damage_fraction * 100.0,
                radius,
                (1.0 - slow_factor) * 100.0,
                slow_duration
            ),
            GemEffect::SlowSplashBoost {
                radius,
                damage_fraction,
                slow_factor,
                slow_duration,
                damage_bonus,
                boost_range,
            } => format!(
                "Splash: {:.0}% r{:.0}; slow -{:.0}% for {:.1}s; boost +{:.0}% r{:.0}",
                damage_fraction * 100.0,
                radius,
                (1.0 - slow_factor) * 100.0,
                slow_duration,
                damage_bonus * 100.0,
                boost_range
            ),
            GemEffect::Poison {
                dps_per_stack,
                max_stacks,
                slow_factor,
                ..
            } => format!(
                "Poison: {:.0}/s up to {}x, -{:.0}% slow",
                dps_per_stack,
                max_stacks,
                (1.0 - slow_factor) * 100.0
            ),
            GemEffect::Multi { targets, .. } => format!("Hits up to {} targets", targets),
            GemEffect::Area { damage_fraction } => {
                if damage_fraction >= 0.995 {
                    "Hits every enemy in range".to_string()
                } else {
                    format!(
                        "Area: {:.0}% to every enemy in range",
                        damage_fraction * 100.0
                    )
                }
            }
            GemEffect::Favored { air, multiplier } => format!(
                "vs {}: x{:.1} damage",
                if air { "air" } else { "ground" },
                multiplier
            ),
            GemEffect::Haste { cooldown_reduction } => {
                format!("Cooldowns -{:.0}% in range", cooldown_reduction * 100.0)
            }
            GemEffect::DamageBoost {
                damage_bonus,
                range,
            } => format!(
                "Mystic boost: +{:.0}% damage once in r{:.0}",
                damage_bonus * 100.0,
                range
            ),
            GemEffect::AncientBlood {
                triple_chance,
                triple_multiplier,
                burn_chance,
                burn_dps,
                burn_duration,
            } => format!(
                "{:.0}% x{:.0}; {:.0}% burn {:.0}/s for {:.0}s",
                triple_chance * 100.0,
                triple_multiplier,
                burn_chance * 100.0,
                burn_dps,
                burn_duration
            ),
            GemEffect::ArmorBreak {
                crit_chance,
                crit_multiplier,
                armor_reduction,
                duration,
            } => format!(
                "{:.0}% x{:.0}; armor -{:.0} for {:.0}s",
                crit_chance * 100.0,
                crit_multiplier,
                armor_reduction,
                duration
            ),
            GemEffect::JadePoison {
                dps,
                duration,
                slow_factor,
                crit_chance,
                crit_multiplier,
                stun_chance,
                gold_chance,
                ..
            } => {
                let base = format!(
                    "Poison: {:.0}/s {:.0}s, -{:.0}% slow",
                    dps,
                    duration,
                    (1.0 - slow_factor) * 100.0
                );
                if crit_chance > 0.0 || stun_chance > 0.0 || gold_chance > 0.0 {
                    format!(
                        "{}; {:.0}% x{:.0}; {:.0}% stun/gold",
                        base,
                        crit_chance * 100.0,
                        crit_multiplier,
                        stun_chance.max(gold_chance) * 100.0
                    )
                } else {
                    base
                }
            }
            GemEffect::Critical {
                chance,
                multiplier,
                ground_only,
            } => format!(
                "{:.0}% x{:.0}{}",
                chance * 100.0,
                multiplier,
                if ground_only { ", ground only" } else { "" }
            ),
            GemEffect::AirArmorAura { armor_reduction } => {
                format!("Air only; flying armor -{:.0} in range", armor_reduction)
            }
            GemEffect::Tourmaline {
                proc_chance,
                proc_total_damage,
                proc_duration,
                armor_reduction,
                armor_range,
            } => format!(
                "{:.0}% {:.0}/{:.0}s; ground armor -{:.0} r{:.0}",
                proc_chance * 100.0,
                proc_total_damage,
                proc_duration,
                armor_reduction,
                armor_range
            ),
            GemEffect::SlowAura { factor, radius, .. } => {
                format!(
                    "Aura slow: -{:.0}% in r{:.0}",
                    (1.0 - factor) * 100.0,
                    radius
                )
            }
            GemEffect::DamageAura { dps } => format!("Aura: {:.0}/s to enemies in range", dps),
        }
    }
}

pub const GEM_KINDS: [GemKind; 8] = [
    GemKind::Ruby,
    GemKind::Sapphire,
    GemKind::Topaz,
    GemKind::Emerald,
    GemKind::Amethyst,
    GemKind::Diamond,
    GemKind::Aquamarine,
    GemKind::Opal,
];

pub const GRADE_LADDER: [GemGrade; 5] = [
    GemGrade::Chipped,
    GemGrade::Flawed,
    GemGrade::Regular,
    GemGrade::Cut,
    GemGrade::Perfect,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GemGrade {
    Chipped,
    Flawed,
    Regular,
    Cut,
    Perfect,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpecialGem {
    BlackOpal,
    MysticBlackOpal,
    BloodStone,
    AncientBloodStone,
    Gold,
    EgyptianGold,
    Jade,
    AsianJade,
    LuckyAsianJade,
    Malachite,
    VividMalachite,
    MightyMalachite,
    PinkDiamond,
    GreatPinkDiamond,
    RedCrystal,
    RedCrystalFacet,
    RoseQuartzCrystal,
    Silver,
    SterlingSilver,
    SilverKnight,
    StarRuby,
    BloodStar,
    FireStar,
    Tourmaline,
    ParaibaTourmaline,
    Uranium238,
    Uranium235,
    YellowSapphire,
    StarYellowSapphire,
}

impl SpecialGem {
    pub const BLACK_OPAL_UPGRADE_COST: u32 = 250;
    pub const BLACK_OPAL_DAMAGE_BOOST: f32 = 0.40;
    pub const BLOOD_STONE_UPGRADE_COST: u32 = 250;
    pub const GOLD_UPGRADE_COST: u32 = 210;
    pub const GOLD_ARMOR_DURATION: f32 = 5.0;
    pub const ASIAN_JADE_UPGRADE_COST: u32 = 45;
    pub const LUCKY_ASIAN_JADE_UPGRADE_COST: u32 = 250;
    pub const VIVID_MALACHITE_UPGRADE_COST: u32 = 25;
    pub const MIGHTY_MALACHITE_UPGRADE_COST: u32 = 280;
    pub const GREAT_PINK_DIAMOND_UPGRADE_COST: u32 = 175;
    pub const RED_CRYSTAL_FACET_UPGRADE_COST: u32 = 100;
    pub const ROSE_QUARTZ_CRYSTAL_UPGRADE_COST: u32 = 150;
    pub const STERLING_SILVER_UPGRADE_COST: u32 = 25;
    pub const SILVER_KNIGHT_UPGRADE_COST: u32 = 300;
    pub const SILVER_SPLASH_RADIUS: f32 = 36.0;
    pub const SILVER_SLOW_FACTOR: f32 = 0.8;
    pub const SILVER_SLOW_DURATION: f32 = 5.0;
    pub const BLOOD_STAR_UPGRADE_COST: u32 = 30;
    pub const FIRE_STAR_UPGRADE_COST: u32 = 290;
    pub const PARAIBA_TOURMALINE_UPGRADE_COST: u32 = 350;
    pub const TOURMALINE_PROC_CHANCE: f32 = 0.20;
    pub const TOURMALINE_PROC_DURATION: f32 = 3.0;
    pub const URANIUM_235_UPGRADE_COST: u32 = 135;
    pub const URANIUM_SLOW_FACTOR: f32 = 0.5;
    pub const URANIUM_SLOW_DURATION: f32 = 0.25;
    pub const STAR_YELLOW_SAPPHIRE_UPGRADE_COST: u32 = 60;
    pub const YELLOW_SAPPHIRE_SPLASH_RADIUS: f32 = 57.0;
    pub const YELLOW_SAPPHIRE_SLOW_DURATION: f32 = 3.0;
    pub const STAR_YELLOW_SAPPHIRE_DAMAGE_BOOST: f32 = 0.05;

    pub fn name(self) -> &'static str {
        match self {
            SpecialGem::BlackOpal => "Black Opal",
            SpecialGem::MysticBlackOpal => "Mystic Black Opal",
            SpecialGem::BloodStone => "Blood Stone",
            SpecialGem::AncientBloodStone => "Ancient Blood Stone",
            SpecialGem::Gold => "Gold",
            SpecialGem::EgyptianGold => "Egyptian Gold",
            SpecialGem::Jade => "Jade",
            SpecialGem::AsianJade => "Asian Jade",
            SpecialGem::LuckyAsianJade => "Lucky Asian Jade",
            SpecialGem::Malachite => "Malachite",
            SpecialGem::VividMalachite => "Vivid Malachite",
            SpecialGem::MightyMalachite => "Mighty Malachite",
            SpecialGem::PinkDiamond => "Pink Diamond",
            SpecialGem::GreatPinkDiamond => "Great Pink Diamond",
            SpecialGem::RedCrystal => "Red Crystal",
            SpecialGem::RedCrystalFacet => "Red Crystal Facet",
            SpecialGem::RoseQuartzCrystal => "Rose Quartz Crystal",
            SpecialGem::Silver => "Silver",
            SpecialGem::SterlingSilver => "Sterling Silver",
            SpecialGem::SilverKnight => "Silver Knight",
            SpecialGem::StarRuby => "Star Ruby",
            SpecialGem::BloodStar => "Blood Star",
            SpecialGem::FireStar => "Fire Star",
            SpecialGem::Tourmaline => "Tourmaline",
            SpecialGem::ParaibaTourmaline => "Paraiba Tourmaline",
            SpecialGem::Uranium238 => "Uranium 238",
            SpecialGem::Uranium235 => "Uranium 235",
            SpecialGem::YellowSapphire => "Yellow Sapphire",
            SpecialGem::StarYellowSapphire => "Star Yellow Sapphire",
        }
    }

    pub fn srgb(self) -> [f32; 3] {
        match self {
            SpecialGem::BlackOpal => [0.05, 0.06, 0.08],
            SpecialGem::MysticBlackOpal => [0.14, 0.10, 0.22],
            SpecialGem::BloodStone => [0.70, 0.06, 0.05],
            SpecialGem::AncientBloodStone => [0.95, 0.20, 0.06],
            SpecialGem::Gold => [1.0, 0.72, 0.10],
            SpecialGem::EgyptianGold => [0.98, 0.86, 0.32],
            SpecialGem::Jade => [0.10, 0.72, 0.42],
            SpecialGem::AsianJade => [0.14, 0.86, 0.58],
            SpecialGem::LuckyAsianJade => [0.30, 1.0, 0.74],
            SpecialGem::Malachite => [0.08, 0.66, 0.28],
            SpecialGem::VividMalachite => [0.10, 0.86, 0.36],
            SpecialGem::MightyMalachite => [0.34, 1.0, 0.42],
            SpecialGem::PinkDiamond => [1.0, 0.42, 0.78],
            SpecialGem::GreatPinkDiamond => [1.0, 0.68, 0.90],
            SpecialGem::RedCrystal => [0.84, 0.05, 0.12],
            SpecialGem::RedCrystalFacet => [0.96, 0.10, 0.18],
            SpecialGem::RoseQuartzCrystal => [1.0, 0.32, 0.45],
            SpecialGem::Silver => [0.76, 0.82, 0.88],
            SpecialGem::SterlingSilver => [0.88, 0.92, 0.96],
            SpecialGem::SilverKnight => [0.92, 0.96, 1.0],
            SpecialGem::StarRuby => [0.88, 0.02, 0.10],
            SpecialGem::BloodStar => [1.0, 0.04, 0.14],
            SpecialGem::FireStar => [1.0, 0.22, 0.04],
            SpecialGem::Tourmaline => [0.10, 0.86, 0.82],
            SpecialGem::ParaibaTourmaline => [0.20, 1.0, 0.94],
            SpecialGem::Uranium238 => [0.58, 0.92, 0.16],
            SpecialGem::Uranium235 => [0.78, 1.0, 0.22],
            SpecialGem::YellowSapphire => [1.0, 0.86, 0.10],
            SpecialGem::StarYellowSapphire => [1.0, 0.96, 0.26],
        }
    }

    pub fn stats(self) -> TowerStats {
        match self {
            // Wiki damage is 24-25; use the average because this prototype has
            // fixed tower damage rather than per-shot damage rolls.
            SpecialGem::BlackOpal => TowerStats::new(24.5, 114.0, 1.0),
            SpecialGem::MysticBlackOpal => TowerStats::new(70.0, 143.0, 1.0),
            SpecialGem::BloodStone => TowerStats::new(68.0, 100.0, 0.5),
            // Wiki damage is 160-240; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::AncientBloodStone => TowerStats::new(200.0, 100.0, 0.8),
            // Wiki damage is 160-190; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::Gold => TowerStats::new(175.0, 114.0, 1.0),
            // Wiki damage is 160-200; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::EgyptianGold => TowerStats::new(180.0, 114.0, 0.75),
            // Wiki damage is 30-35; use the average because this prototype has
            // fixed tower damage rather than per-shot damage rolls.
            SpecialGem::Jade => TowerStats::new(32.5, 114.0, 0.5),
            SpecialGem::AsianJade => TowerStats::new(50.0, 114.0, 0.5),
            SpecialGem::LuckyAsianJade => TowerStats::new(55.0, 121.0, 0.35),
            SpecialGem::Malachite => TowerStats::new(6.0, 107.0, 0.5),
            SpecialGem::VividMalachite => TowerStats::new(11.0, 114.0, 0.5),
            SpecialGem::MightyMalachite => TowerStats::new(45.0, 114.0, 0.55),
            // Wiki damage is 150-175; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::PinkDiamond => TowerStats::new(162.5, 114.0, 0.75),
            // Wiki damage is 175-225; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::GreatPinkDiamond => TowerStats::new(200.0, 121.0, 0.65),
            // Wiki damage is 50-75; use the average because this prototype has
            // fixed tower damage rather than per-shot damage rolls.
            SpecialGem::RedCrystal => TowerStats::new(62.5, 186.0, 0.8),
            // Wiki damage is 75-100; use the average because this prototype has
            // fixed tower damage rather than per-shot damage rolls.
            SpecialGem::RedCrystalFacet => TowerStats::new(87.5, 200.0, 0.8),
            // Wiki damage is 100-125; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::RoseQuartzCrystal => TowerStats::new(112.5, 214.0, 0.8),
            // atwiki lists Silver as 20-21; use the average because this
            // prototype has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::Silver => TowerStats::new(20.5, 78.0, 1.0),
            SpecialGem::SterlingSilver => TowerStats::new(40.0, 92.0, 1.0),
            SpecialGem::SilverKnight => TowerStats::new(150.0, 107.0, 1.0),
            SpecialGem::StarRuby => TowerStats::new(40.0, 38.0, 1.0),
            SpecialGem::BloodStar => TowerStats::new(50.0, 71.0, 1.0),
            SpecialGem::FireStar => TowerStats::new(130.0, 85.0, 1.0),
            // Wiki damage is 10-400; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::Tourmaline => TowerStats::new(205.0, 121.0, 0.75),
            // Wiki damage is 30-420; use the average because this prototype
            // has fixed tower damage rather than per-shot damage rolls.
            SpecialGem::ParaibaTourmaline => TowerStats::new(225.0, 125.0, 0.75),
            SpecialGem::Uranium238 => TowerStats::new(47.0, 64.0, 0.25),
            SpecialGem::Uranium235 => TowerStats::new(64.0, 85.0, 0.25),
            SpecialGem::YellowSapphire => TowerStats::new(100.0, 125.0, 1.25),
            SpecialGem::StarYellowSapphire => TowerStats::new(220.0, 132.0, 1.25),
        }
    }

    pub fn effect(self) -> GemEffect {
        match self {
            SpecialGem::BlackOpal => GemEffect::None,
            SpecialGem::MysticBlackOpal => GemEffect::DamageBoost {
                damage_bonus: Self::BLACK_OPAL_DAMAGE_BOOST,
                range: TowerStats::scaled_range(171.0),
            },
            SpecialGem::BloodStone => GemEffect::Area {
                damage_fraction: 1.0,
            },
            SpecialGem::AncientBloodStone => GemEffect::AncientBlood {
                triple_chance: 0.15,
                triple_multiplier: 3.0,
                burn_chance: 0.10,
                burn_dps: 500.0,
                burn_duration: 5.0,
            },
            SpecialGem::Gold => GemEffect::ArmorBreak {
                crit_chance: 0.25,
                crit_multiplier: 2.0,
                armor_reduction: 5.0,
                duration: Self::GOLD_ARMOR_DURATION,
            },
            SpecialGem::EgyptianGold => GemEffect::ArmorBreak {
                crit_chance: 0.30,
                crit_multiplier: 2.0,
                armor_reduction: 8.0,
                duration: Self::GOLD_ARMOR_DURATION,
            },
            SpecialGem::Jade => GemEffect::JadePoison {
                dps: 5.0,
                duration: 2.0,
                slow_factor: 0.5,
                crit_chance: 0.0,
                crit_multiplier: 1.0,
                stun_chance: 0.0,
                stun_duration: 0.0,
                gold_chance: 0.0,
            },
            SpecialGem::AsianJade => GemEffect::JadePoison {
                dps: 10.0,
                duration: 3.0,
                slow_factor: 0.5,
                crit_chance: 0.0,
                crit_multiplier: 1.0,
                stun_chance: 0.0,
                stun_duration: 0.0,
                gold_chance: 0.0,
            },
            SpecialGem::LuckyAsianJade => GemEffect::JadePoison {
                dps: 10.0,
                duration: 4.0,
                slow_factor: 0.5,
                crit_chance: 0.05,
                crit_multiplier: 4.0,
                stun_chance: 0.01,
                stun_duration: 2.0,
                gold_chance: 0.01,
            },
            SpecialGem::Malachite => GemEffect::Multi {
                targets: 3,
                damage_fraction: 1.0,
            },
            SpecialGem::VividMalachite => GemEffect::Multi {
                targets: 4,
                damage_fraction: 1.0,
            },
            SpecialGem::MightyMalachite => GemEffect::Area {
                damage_fraction: 1.0,
            },
            SpecialGem::PinkDiamond => GemEffect::Critical {
                chance: 0.10,
                multiplier: 5.0,
                ground_only: true,
            },
            SpecialGem::GreatPinkDiamond => GemEffect::Critical {
                chance: 0.10,
                multiplier: 8.0,
                ground_only: true,
            },
            SpecialGem::RedCrystal => GemEffect::AirArmorAura {
                armor_reduction: 4.0,
            },
            SpecialGem::RedCrystalFacet => GemEffect::AirArmorAura {
                armor_reduction: 5.0,
            },
            SpecialGem::RoseQuartzCrystal => GemEffect::AirArmorAura {
                armor_reduction: 6.0,
            },
            SpecialGem::Silver | SpecialGem::SterlingSilver | SpecialGem::SilverKnight => {
                GemEffect::SlowSplash {
                    radius: Self::SILVER_SPLASH_RADIUS,
                    damage_fraction: 1.0,
                    slow_factor: Self::SILVER_SLOW_FACTOR,
                    slow_duration: Self::SILVER_SLOW_DURATION,
                }
            }
            SpecialGem::StarRuby => GemEffect::DamageAura { dps: 40.0 },
            SpecialGem::BloodStar => GemEffect::DamageAura { dps: 50.0 },
            SpecialGem::FireStar => GemEffect::DamageAura { dps: 130.0 },
            SpecialGem::Tourmaline => GemEffect::Tourmaline {
                proc_chance: Self::TOURMALINE_PROC_CHANCE,
                proc_total_damage: 200.0,
                proc_duration: Self::TOURMALINE_PROC_DURATION,
                armor_reduction: 4.0,
                armor_range: TowerStats::scaled_range(85.8),
            },
            SpecialGem::ParaibaTourmaline => GemEffect::Tourmaline {
                proc_chance: Self::TOURMALINE_PROC_CHANCE,
                proc_total_damage: 250.0,
                proc_duration: Self::TOURMALINE_PROC_DURATION,
                armor_reduction: 6.0,
                armor_range: TowerStats::scaled_range(93.0),
            },
            SpecialGem::Uranium238 => GemEffect::SlowAura {
                factor: Self::URANIUM_SLOW_FACTOR,
                radius: TowerStats::scaled_range(64.0),
                duration: Self::URANIUM_SLOW_DURATION,
                ignores_opals: true,
            },
            SpecialGem::Uranium235 => GemEffect::SlowAura {
                factor: Self::URANIUM_SLOW_FACTOR,
                radius: TowerStats::scaled_range(64.0),
                duration: Self::URANIUM_SLOW_DURATION,
                ignores_opals: true,
            },
            SpecialGem::YellowSapphire => GemEffect::SlowSplash {
                radius: Self::YELLOW_SAPPHIRE_SPLASH_RADIUS,
                damage_fraction: 1.0,
                slow_factor: 0.70,
                slow_duration: Self::YELLOW_SAPPHIRE_SLOW_DURATION,
            },
            SpecialGem::StarYellowSapphire => GemEffect::SlowSplashBoost {
                radius: Self::YELLOW_SAPPHIRE_SPLASH_RADIUS,
                damage_fraction: 1.0,
                slow_factor: 0.60,
                slow_duration: Self::YELLOW_SAPPHIRE_SLOW_DURATION,
                damage_bonus: Self::STAR_YELLOW_SAPPHIRE_DAMAGE_BOOST,
                boost_range: TowerStats::scaled_range(171.0),
            },
        }
    }

    pub fn upgrade(self) -> Option<SpecialGem> {
        match self {
            SpecialGem::BlackOpal => Some(SpecialGem::MysticBlackOpal),
            SpecialGem::BloodStone => Some(SpecialGem::AncientBloodStone),
            SpecialGem::Gold => Some(SpecialGem::EgyptianGold),
            SpecialGem::Jade => Some(SpecialGem::AsianJade),
            SpecialGem::AsianJade => Some(SpecialGem::LuckyAsianJade),
            SpecialGem::Malachite => Some(SpecialGem::VividMalachite),
            SpecialGem::VividMalachite => Some(SpecialGem::MightyMalachite),
            SpecialGem::PinkDiamond => Some(SpecialGem::GreatPinkDiamond),
            SpecialGem::RedCrystal => Some(SpecialGem::RedCrystalFacet),
            SpecialGem::RedCrystalFacet => Some(SpecialGem::RoseQuartzCrystal),
            SpecialGem::Silver => Some(SpecialGem::SterlingSilver),
            SpecialGem::SterlingSilver => Some(SpecialGem::SilverKnight),
            SpecialGem::StarRuby => Some(SpecialGem::BloodStar),
            SpecialGem::BloodStar => Some(SpecialGem::FireStar),
            SpecialGem::Tourmaline => Some(SpecialGem::ParaibaTourmaline),
            SpecialGem::Uranium238 => Some(SpecialGem::Uranium235),
            SpecialGem::YellowSapphire => Some(SpecialGem::StarYellowSapphire),
            SpecialGem::MysticBlackOpal
            | SpecialGem::AncientBloodStone
            | SpecialGem::EgyptianGold
            | SpecialGem::LuckyAsianJade
            | SpecialGem::MightyMalachite
            | SpecialGem::GreatPinkDiamond
            | SpecialGem::RoseQuartzCrystal
            | SpecialGem::SilverKnight
            | SpecialGem::FireStar
            | SpecialGem::ParaibaTourmaline
            | SpecialGem::Uranium235
            | SpecialGem::StarYellowSapphire => None,
        }
    }

    pub fn upgrade_cost(self) -> Option<u32> {
        match self {
            SpecialGem::BlackOpal => Some(Self::BLACK_OPAL_UPGRADE_COST),
            SpecialGem::BloodStone => Some(Self::BLOOD_STONE_UPGRADE_COST),
            SpecialGem::Gold => Some(Self::GOLD_UPGRADE_COST),
            SpecialGem::Jade => Some(Self::ASIAN_JADE_UPGRADE_COST),
            SpecialGem::AsianJade => Some(Self::LUCKY_ASIAN_JADE_UPGRADE_COST),
            SpecialGem::Malachite => Some(Self::VIVID_MALACHITE_UPGRADE_COST),
            SpecialGem::VividMalachite => Some(Self::MIGHTY_MALACHITE_UPGRADE_COST),
            SpecialGem::PinkDiamond => Some(Self::GREAT_PINK_DIAMOND_UPGRADE_COST),
            SpecialGem::RedCrystal => Some(Self::RED_CRYSTAL_FACET_UPGRADE_COST),
            SpecialGem::RedCrystalFacet => Some(Self::ROSE_QUARTZ_CRYSTAL_UPGRADE_COST),
            SpecialGem::Silver => Some(Self::STERLING_SILVER_UPGRADE_COST),
            SpecialGem::SterlingSilver => Some(Self::SILVER_KNIGHT_UPGRADE_COST),
            SpecialGem::StarRuby => Some(Self::BLOOD_STAR_UPGRADE_COST),
            SpecialGem::BloodStar => Some(Self::FIRE_STAR_UPGRADE_COST),
            SpecialGem::Tourmaline => Some(Self::PARAIBA_TOURMALINE_UPGRADE_COST),
            SpecialGem::Uranium238 => Some(Self::URANIUM_235_UPGRADE_COST),
            SpecialGem::YellowSapphire => Some(Self::STAR_YELLOW_SAPPHIRE_UPGRADE_COST),
            SpecialGem::MysticBlackOpal
            | SpecialGem::AncientBloodStone
            | SpecialGem::EgyptianGold
            | SpecialGem::LuckyAsianJade
            | SpecialGem::MightyMalachite
            | SpecialGem::GreatPinkDiamond
            | SpecialGem::RoseQuartzCrystal
            | SpecialGem::SilverKnight
            | SpecialGem::FireStar
            | SpecialGem::ParaibaTourmaline
            | SpecialGem::Uranium235
            | SpecialGem::StarYellowSapphire => None,
        }
    }

    pub fn sprite_grade(self) -> GemGrade {
        match self {
            SpecialGem::BlackOpal => GemGrade::Cut,
            SpecialGem::MysticBlackOpal => GemGrade::Perfect,
            SpecialGem::BloodStone => GemGrade::Cut,
            SpecialGem::AncientBloodStone => GemGrade::Perfect,
            SpecialGem::Gold => GemGrade::Cut,
            SpecialGem::EgyptianGold => GemGrade::Perfect,
            SpecialGem::Jade => GemGrade::Regular,
            SpecialGem::AsianJade => GemGrade::Cut,
            SpecialGem::LuckyAsianJade => GemGrade::Perfect,
            SpecialGem::Malachite => GemGrade::Chipped,
            SpecialGem::VividMalachite => GemGrade::Cut,
            SpecialGem::MightyMalachite => GemGrade::Perfect,
            SpecialGem::PinkDiamond => GemGrade::Cut,
            SpecialGem::GreatPinkDiamond => GemGrade::Perfect,
            SpecialGem::RedCrystal => GemGrade::Cut,
            SpecialGem::RedCrystalFacet => GemGrade::Cut,
            SpecialGem::RoseQuartzCrystal => GemGrade::Perfect,
            SpecialGem::Silver => GemGrade::Chipped,
            SpecialGem::SterlingSilver => GemGrade::Cut,
            SpecialGem::SilverKnight => GemGrade::Perfect,
            SpecialGem::StarRuby => GemGrade::Chipped,
            SpecialGem::BloodStar => GemGrade::Cut,
            SpecialGem::FireStar => GemGrade::Perfect,
            SpecialGem::Tourmaline => GemGrade::Cut,
            SpecialGem::ParaibaTourmaline => GemGrade::Perfect,
            SpecialGem::Uranium238 => GemGrade::Cut,
            SpecialGem::Uranium235 => GemGrade::Perfect,
            SpecialGem::YellowSapphire => GemGrade::Cut,
            SpecialGem::StarYellowSapphire => GemGrade::Perfect,
        }
    }
}

pub const SPECIAL_GEMS: [SpecialGem; 29] = [
    SpecialGem::BlackOpal,
    SpecialGem::MysticBlackOpal,
    SpecialGem::BloodStone,
    SpecialGem::AncientBloodStone,
    SpecialGem::Gold,
    SpecialGem::EgyptianGold,
    SpecialGem::Jade,
    SpecialGem::AsianJade,
    SpecialGem::LuckyAsianJade,
    SpecialGem::Malachite,
    SpecialGem::VividMalachite,
    SpecialGem::MightyMalachite,
    SpecialGem::PinkDiamond,
    SpecialGem::GreatPinkDiamond,
    SpecialGem::RedCrystal,
    SpecialGem::RedCrystalFacet,
    SpecialGem::RoseQuartzCrystal,
    SpecialGem::Silver,
    SpecialGem::SterlingSilver,
    SpecialGem::SilverKnight,
    SpecialGem::StarRuby,
    SpecialGem::BloodStar,
    SpecialGem::FireStar,
    SpecialGem::Tourmaline,
    SpecialGem::ParaibaTourmaline,
    SpecialGem::Uranium238,
    SpecialGem::Uranium235,
    SpecialGem::YellowSapphire,
    SpecialGem::StarYellowSapphire,
];

#[derive(Clone, Copy)]
pub struct SpecialRecipe {
    pub result: SpecialGem,
    pub source: (GemKind, GemGrade),
    pub components: &'static [(GemKind, GemGrade)],
}

impl SpecialRecipe {
    pub fn ingredient_summary(self) -> String {
        let mut parts = Vec::with_capacity(self.components.len() + 1);
        parts.push(recipe_piece_name(self.source));
        parts.extend(self.components.iter().copied().map(recipe_piece_name));
        parts.join(", ")
    }
}

fn recipe_piece_name((gem, grade): (GemKind, GemGrade)) -> String {
    format!("{} {}", grade.name(), gem.name())
}

/// Special recipes from the Gem Tower Defense wiki. The wiki's ladder is
/// Flawed / Normal / Flawless / Perfect; this prototype names those middle
/// grades Flawed / Regular / Cut / Perfect.
pub const BLACK_OPAL_SOURCE: (GemKind, GemGrade) = (GemKind::Opal, GemGrade::Perfect);
pub const BLACK_OPAL_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Diamond, GemGrade::Cut),
    (GemKind::Aquamarine, GemGrade::Regular),
];
pub const BLOOD_STONE_SOURCE: (GemKind, GemGrade) = (GemKind::Ruby, GemGrade::Perfect);
pub const BLOOD_STONE_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Aquamarine, GemGrade::Cut),
    (GemKind::Amethyst, GemGrade::Regular),
];
pub const GOLD_SOURCE: (GemKind, GemGrade) = (GemKind::Amethyst, GemGrade::Perfect);
pub const GOLD_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Amethyst, GemGrade::Cut),
    (GemKind::Diamond, GemGrade::Flawed),
];
pub const JADE_SOURCE: (GemKind, GemGrade) = (GemKind::Emerald, GemGrade::Regular);
pub const JADE_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Opal, GemGrade::Regular),
    (GemKind::Sapphire, GemGrade::Flawed),
];
pub const MALACHITE_SOURCE: (GemKind, GemGrade) = (GemKind::Opal, GemGrade::Chipped);
pub const MALACHITE_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Emerald, GemGrade::Chipped),
    (GemKind::Aquamarine, GemGrade::Chipped),
];
pub const PINK_DIAMOND_SOURCE: (GemKind, GemGrade) = (GemKind::Diamond, GemGrade::Perfect);
pub const PINK_DIAMOND_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Diamond, GemGrade::Regular),
    (GemKind::Topaz, GemGrade::Regular),
];
pub const RED_CRYSTAL_SOURCE: (GemKind, GemGrade) = (GemKind::Emerald, GemGrade::Cut);
pub const RED_CRYSTAL_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Ruby, GemGrade::Regular),
    (GemKind::Amethyst, GemGrade::Flawed),
];
pub const SILVER_SOURCE: (GemKind, GemGrade) = (GemKind::Sapphire, GemGrade::Chipped);
pub const SILVER_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Diamond, GemGrade::Chipped),
    (GemKind::Topaz, GemGrade::Chipped),
];
pub const STAR_RUBY_SOURCE: (GemKind, GemGrade) = (GemKind::Amethyst, GemGrade::Chipped);
pub const STAR_RUBY_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Ruby, GemGrade::Chipped),
    (GemKind::Ruby, GemGrade::Flawed),
];
pub const TOURMALINE_SOURCE: (GemKind, GemGrade) = (GemKind::Aquamarine, GemGrade::Perfect);
pub const TOURMALINE_COMPONENTS: [(GemKind, GemGrade); 3] = [
    (GemKind::Opal, GemGrade::Cut),
    (GemKind::Aquamarine, GemGrade::Flawed),
    (GemKind::Emerald, GemGrade::Flawed),
];
pub const URANIUM_SOURCE: (GemKind, GemGrade) = (GemKind::Topaz, GemGrade::Perfect);
pub const URANIUM_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Opal, GemGrade::Flawed),
    (GemKind::Sapphire, GemGrade::Regular),
];
pub const YELLOW_SAPPHIRE_SOURCE: (GemKind, GemGrade) = (GemKind::Sapphire, GemGrade::Perfect);
pub const YELLOW_SAPPHIRE_COMPONENTS: [(GemKind, GemGrade); 2] = [
    (GemKind::Ruby, GemGrade::Cut),
    (GemKind::Topaz, GemGrade::Cut),
];
pub const SPECIAL_RECIPES: [SpecialRecipe; 12] = [
    SpecialRecipe {
        result: SpecialGem::BlackOpal,
        source: BLACK_OPAL_SOURCE,
        components: &BLACK_OPAL_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::BloodStone,
        source: BLOOD_STONE_SOURCE,
        components: &BLOOD_STONE_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Gold,
        source: GOLD_SOURCE,
        components: &GOLD_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Jade,
        source: JADE_SOURCE,
        components: &JADE_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Malachite,
        source: MALACHITE_SOURCE,
        components: &MALACHITE_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::PinkDiamond,
        source: PINK_DIAMOND_SOURCE,
        components: &PINK_DIAMOND_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::RedCrystal,
        source: RED_CRYSTAL_SOURCE,
        components: &RED_CRYSTAL_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Silver,
        source: SILVER_SOURCE,
        components: &SILVER_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::StarRuby,
        source: STAR_RUBY_SOURCE,
        components: &STAR_RUBY_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Tourmaline,
        source: TOURMALINE_SOURCE,
        components: &TOURMALINE_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::Uranium238,
        source: URANIUM_SOURCE,
        components: &URANIUM_COMPONENTS,
    },
    SpecialRecipe {
        result: SpecialGem::YellowSapphire,
        source: YELLOW_SAPPHIRE_SOURCE,
        components: &YELLOW_SAPPHIRE_COMPONENTS,
    },
];

pub fn special_recipe_for_source(gem: GemKind, grade: GemGrade) -> Option<SpecialRecipe> {
    SPECIAL_RECIPES
        .iter()
        .copied()
        .find(|recipe| recipe.source == (gem, grade))
}

impl GemGrade {
    pub fn name(self) -> &'static str {
        match self {
            GemGrade::Chipped => "Chipped",
            GemGrade::Flawed => "Flawed",
            GemGrade::Regular => "Regular",
            GemGrade::Cut => "Cut",
            GemGrade::Perfect => "Perfect",
        }
    }

    /// Position on the `GRADE_LADDER`, `0` (Chipped) through `4` (Perfect).
    pub fn tier(self) -> usize {
        match self {
            GemGrade::Chipped => 0,
            GemGrade::Flawed => 1,
            GemGrade::Regular => 2,
            GemGrade::Cut => 3,
            GemGrade::Perfect => 4,
        }
    }

    pub fn next(self) -> Option<GemGrade> {
        match self {
            GemGrade::Chipped => Some(GemGrade::Flawed),
            GemGrade::Flawed => Some(GemGrade::Regular),
            GemGrade::Regular => Some(GemGrade::Cut),
            GemGrade::Cut => Some(GemGrade::Perfect),
            GemGrade::Perfect => None,
        }
    }

    pub fn damage_multiplier(self) -> f32 {
        match self {
            GemGrade::Chipped => 1.0,
            GemGrade::Flawed => 1.45,
            GemGrade::Regular => 2.05,
            GemGrade::Cut => 2.9,
            GemGrade::Perfect => 4.1,
        }
    }

    pub fn size_multiplier(self) -> f32 {
        match self {
            GemGrade::Chipped => 1.0,
            GemGrade::Flawed => 1.08,
            GemGrade::Regular => 1.16,
            GemGrade::Cut => 1.24,
            GemGrade::Perfect => 1.34,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TowerStats {
    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
}

impl TowerStats {
    pub fn new(damage: f32, range: f32, cooldown: f32) -> Self {
        Self {
            damage,
            range: Self::scaled_range(range),
            cooldown,
        }
    }

    pub fn scaled_range(range: f32) -> f32 {
        range * TOWER_RANGE_MULTIPLIER
    }
}

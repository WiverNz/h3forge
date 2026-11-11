//! Typed hero edit operations and patch planning (H3F-007).
//!
//! Operations are semantic ([`HeroEdit`]), never offsets: each one translates
//! into checked [`BytePatch`] values against the document's own payload, with
//! the current bytes as the expected preimage. Coupling rules ported from
//! h3sed `hero/spells.py:serialize` and `hero/stats.py` (MIT, copyright 2020
//! Erki Suurjaak; see licenses/h3sed-MIT.txt):
//!
//! - learning a spell sets the learned flag, and also the available flag when
//!   the hero carries a spellbook (h3sed `in_book = has_spellbook && learned`);
//! - unlearning clears the learned flag and, with a spellbook, the available
//!   flag. Artifact-granted spells (Spell Scrolls) are not decoded yet, so a
//!   scroll-granted spell may need a re-equip to re-grant after unlearning;
//! - level and experience are stored independently and never synchronized
//!   implicitly; `EXPERIENCE_LEVEL_MIN` backs only explicit UI actions;
//! - primary attributes use the conservative HotA game-valid ranges from
//!   h3sed `version/hota.py` (`PRIMARY_ATTRIBUTE_GAME_RANGES`), not the raw
//!   overflow domain: values at 100+ have cap/handicap semantics the editor
//!   does not write in this milestone;
//! - equipment edits (H3F-012) rewrite the stored attributes by the artifact
//!   bonus delta and the available flags by the artifact's spell grants
//!   (h3sed `update_primary_stats` / `spells.py:serialize`), never the learned
//!   flags, and are blocked outright for combination-artifact, scroll,
//!   unknown or reservation-byte state.
//!
//! Every no-op operation produces no patch; the payload outside the patch
//! union is byte-identical by construction.

use crate::definitions::hero::{
    ARMY_COUNTS, ARMY_IDS, ATTACK, DEFENSE, EXPERIENCE, KNOWLEDGE, LEVEL, SPELL_FLAG_COUNT,
    SPELL_POINTS, SPELL_POWER, SPELLS_AVAILABLE, SPELLS_LEARNED,
};
use crate::definitions::names::spell_name;
use crate::domain::HeroConfidence;
use crate::error::EditError;
use crate::patch::{BytePatch, apply_patches, compress_gzip};

/// HotA in-game-valid primary attribute ranges (h3sed `version/hota.py`
/// `PRIMARY_ATTRIBUTE_GAME_RANGES`): might allows 0, magic requires at least 1.
const PRIMARY_RANGE_MIGHT: std::ops::RangeInclusive<u8> = 0..=99;
const PRIMARY_RANGE_MAGIC: std::ops::RangeInclusive<u8> = 1..=99;

/// HotA stored level cap (h3sed `version/hota.py` `DATA_RANGES["level"]`,
/// base game allows 75; the reference caps HotA at 74).
const LEVEL_MAX: u8 = 74;

/// A semantic, version-gated hero modification. Values are absolute desired
/// states, never deltas, so replanning against the immutable source payload
/// is idempotent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeroEdit {
    /// Attack primary attribute, HotA game-valid range 0-99.
    SetAttack(u8),
    /// Defense primary attribute, HotA game-valid range 0-99.
    SetDefense(u8),
    /// Spell Power primary attribute, HotA game-valid range 1-99.
    SetSpellPower(u8),
    /// Knowledge primary attribute, HotA game-valid range 1-99.
    SetKnowledge(u8),
    /// Experience points, u32 little-endian; stored independently of level.
    SetExperience(u32),
    /// Hero level, HotA cap 74; stored independently of experience.
    SetLevel(u8),
    /// Spell points remaining, u16 little-endian.
    SetSpellPoints(u16),
    /// Add (`learned = true`) or remove one learned-spell flag, with the
    /// coupled available-flag rule documented in the module header.
    SetSpellLearned { spell_id: u8, learned: bool },
    /// Replace one zero-based hero army slot. None + zero clears it; occupied
    /// counts are 1..=u32::MAX (h3sed metadata DATA_RANGES). No town writes.
    SetArmySlot {
        slot: u8,
        creature_id: Option<u32>,
        count: u32,
    },
    /// Replace the secondary-skill set with this absolute ordered list of
    /// (skill ID, mastery 1-3), h3sed `hero/skills.py:serialize` semantics
    /// made conservative: listed skills get dense one-based order bytes,
    /// base-record skills missing from the list are cleared, and bytes the
    /// parser does not recognize (plus the obsolete legacy region at
    /// 212..238) are never touched.
    SetSkills { skills: Vec<(u8, u8)> },
    /// Replace one zero-based backpack slot (0-63). `Empty` writes h3sed's
    /// canonical blank (`FF FF FF FF 00 00 00 00`); untouched slots keep
    /// their original blank encoding exactly. Slots occupied by unknown
    /// entries stay read-only.
    SetBackpackSlot { slot: u8, item: BackpackEdit },
    /// Replace one equipment slot. Only wear slots (helm through side5) are
    /// editable; the artifact must be writable and fit the slot. The stored
    /// primary attributes are rewritten by the artifact's bonus delta and
    /// the available-spell flags by the artifact's spell grants, exactly as
    /// h3sed `hero/stats.py:update_primary_stats` and
    /// `hero/spells.py:serialize` do; learned flags are never touched.
    /// Moving artifacts between equipment and backpack is composed by the
    /// caller from this plus `SetBackpackSlot` edits.
    SetEquipmentSlot { slot: u8, item: EquipmentEdit },
}

/// Desired content of one equipment slot for [`HeroEdit::SetEquipmentSlot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentEdit {
    Empty,
    /// Verified artifact ID from `crate::artifacts`, writable and slot-compatible.
    Artifact(u32),
}
/// Desired content of one backpack slot for [`HeroEdit::SetBackpackSlot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpackEdit {
    Empty,
    /// Verified artifact ID from `crate::artifacts` (not the scroll or
    /// spellbook IDs, which need special representations).
    Artifact(u32),
    /// Spell scroll carrying this game spell ID (0-69).
    Scroll {
        spell_id: u8,
    },
}

/// A semantic, version-gated town modification (H3F-013). Values are absolute
/// desired states, never deltas, so replanning against the immutable source
/// payload is idempotent. Only town garrisons are writable: a garrison owned
/// by a garrisoned hero is rejected (that hero's own army is the garrison).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TownEdit {
    /// Replace one zero-based garrison slot. `None` + zero clears it;
    /// occupied counts are 1..=u32::MAX (h3sed metadata DATA_RANGES). The
    /// same word layout and empty-encoding preservation as hero armies
    /// apply (h3sed `common/army.py:serialize` serves both).
    SetGarrisonSlot {
        slot: u8,
        creature_id: Option<u32>,
        count: u32,
    },
    /// Desired logical built-building mask of the town (H3F-022), one bit
    /// per building. The planner validates every changed bit, then rewrites
    /// the built mask, the visible-graphic mask, the Mage Guild level byte
    /// and the guild spell counts together.
    SetBuildings { built: u64 },
    /// Desired spell list of one Mage Guild level 1-5 (H3F-022): the spells
    /// in slot order, at most the level's capacity. Remaining slots become
    /// empty and the level's spell count follows when the level is built.
    SetGuildSpells { level: u8, spells: Vec<u8> },
}

impl crate::SaveDocument {
    /// Plans the checked patches for `edits` against this document's
    /// immutable payload. An empty `edits` list, or one whose values all
    /// equal the stored bytes, yields an empty plan.
    pub fn plan_hero_edits(
        &self,
        hero_id: usize,
        edits: &[HeroEdit],
    ) -> Result<Vec<BytePatch>, EditError> {
        if !self.format.hero_layout_supported() {
            return Err(EditError::UnsupportedFormat {
                family: self.format.family_title(),
                major: self.format.major,
                minor: self.format.minor,
            });
        }
        let hero = self
            .heroes
            .get(hero_id)
            .ok_or(EditError::HeroNotFound { id: hero_id })?;
        if hero.confidence != HeroConfidence::High {
            return Err(EditError::RecordNotEditable { id: hero_id });
        }
        let base = hero.span.start;
        let record = &self.payload[hero.span.clone()];
        let mut patches = Vec::new();
        let mut equipment_edits: Vec<(u8, EquipmentEdit)> = Vec::new();

        for edit in edits {
            match edit {
                HeroEdit::SetArmySlot {
                    slot,
                    creature_id,
                    count,
                } => {
                    if !self.format.army_edit_supported() {
                        return Err(EditError::UnsupportedFormat {
                            family: self.format.family_title(),
                            major: self.format.major,
                            minor: self.format.minor,
                        });
                    }
                    plan_stack_slot(
                        &mut patches,
                        base,
                        ARMY_IDS,
                        ARMY_COUNTS,
                        record,
                        &hero.army,
                        *slot,
                        *creature_id,
                        *count,
                        "army slot",
                    )?;
                }
                HeroEdit::SetAttack(value) => {
                    push_byte(
                        &mut patches,
                        base,
                        ATTACK,
                        record,
                        *value,
                        PRIMARY_RANGE_MIGHT,
                        "attack",
                    )?;
                }
                HeroEdit::SetDefense(value) => {
                    push_byte(
                        &mut patches,
                        base,
                        DEFENSE,
                        record,
                        *value,
                        PRIMARY_RANGE_MIGHT,
                        "defense",
                    )?;
                }
                HeroEdit::SetSpellPower(value) => {
                    push_byte(
                        &mut patches,
                        base,
                        SPELL_POWER,
                        record,
                        *value,
                        PRIMARY_RANGE_MAGIC,
                        "spell power",
                    )?;
                }
                HeroEdit::SetKnowledge(value) => {
                    push_byte(
                        &mut patches,
                        base,
                        KNOWLEDGE,
                        record,
                        *value,
                        PRIMARY_RANGE_MAGIC,
                        "knowledge",
                    )?;
                }
                HeroEdit::SetExperience(value) => {
                    push_scalar(
                        &mut patches,
                        base,
                        EXPERIENCE,
                        value.to_le_bytes(),
                        record,
                        "experience",
                    )?;
                }
                HeroEdit::SetLevel(value) => {
                    if !(1..=LEVEL_MAX).contains(value) {
                        return Err(EditError::InvalidValue {
                            field: "level",
                            reason: format!("{value} is outside the HotA range 1-{LEVEL_MAX}"),
                        });
                    }
                    push_byte(
                        &mut patches,
                        base,
                        LEVEL,
                        record,
                        *value,
                        0..=u8::MAX,
                        "level",
                    )?;
                }
                HeroEdit::SetSpellPoints(value) => {
                    push_scalar(
                        &mut patches,
                        base,
                        SPELL_POINTS,
                        value.to_le_bytes(),
                        record,
                        "spell points",
                    )?;
                }
                HeroEdit::SetSpellLearned { spell_id, learned } => {
                    if *spell_id as usize >= SPELL_FLAG_COUNT {
                        return Err(EditError::InvalidValue {
                            field: "spell",
                            reason: format!("spell ID {spell_id} has no flag slot"),
                        });
                    }
                    let learned_at = SPELLS_LEARNED + *spell_id as usize;
                    let available_at = SPELLS_AVAILABLE + *spell_id as usize;
                    let reason = match spell_name(*spell_id as usize) {
                        Some(name) => format!("spell {} ({name})", *spell_id),
                        None => format!("spell {}", *spell_id),
                    };
                    push_scalar(
                        &mut patches,
                        base,
                        learned_at,
                        [*learned as u8],
                        record,
                        &reason,
                    )?;
                    // Available-flag coupling: mirror h3sed's
                    // `available = has_spellbook && in_spellbook` for heroes
                    // carrying a spellbook; without one the available array
                    // is left untouched (artifact grants not decoded yet).
                    if hero.has_spellbook && record[available_at] != *learned as u8 {
                        patches.push(BytePatch::new(
                            base + available_at,
                            [record[available_at]],
                            [*learned as u8],
                            format!("available flag for {reason}"),
                        )?);
                    }
                }
                HeroEdit::SetSkills { skills } => {
                    if !self.format.army_edit_supported() {
                        return Err(EditError::UnsupportedFormat {
                            family: self.format.family_title(),
                            major: self.format.major,
                            minor: self.format.minor,
                        });
                    }
                    plan_skills(&mut patches, base, record, skills)?;
                }
                HeroEdit::SetBackpackSlot { slot, item } => {
                    if !self.format.army_edit_supported() {
                        return Err(EditError::UnsupportedFormat {
                            family: self.format.family_title(),
                            major: self.format.major,
                            minor: self.format.minor,
                        });
                    }
                    plan_backpack_slot(&mut patches, base, record, hero, *slot, *item)?;
                }
                HeroEdit::SetEquipmentSlot { slot, item } => {
                    if !self.format.army_edit_supported() {
                        return Err(EditError::UnsupportedFormat {
                            family: self.format.family_title(),
                            major: self.format.major,
                            minor: self.format.minor,
                        });
                    }
                    // Equipment edits are planned together below: stat
                    // bonuses, spell grants and slot reservations depend on
                    // the whole desired equipment, not on one slot.
                    match equipment_edits.iter().find(|&&(other, _)| other == *slot) {
                        Some(&(_, other)) if other != *item => {
                            return Err(EditError::InvalidValue {
                                field: "edit batch",
                                reason: format!(
                                    "conflicting edits: equipment slot {} is set twice with \
                                     different items; revert one of them",
                                    *slot + 1
                                ),
                            });
                        }
                        Some(_) => {}
                        None => equipment_edits.push((*slot, *item)),
                    }
                }
            }
        }
        plan_equipment(&mut patches, base, record, hero, &equipment_edits)?;
        resolve_patch_conflicts(patches)
    }

    /// Builds the complete modified save: applies `patches` to a copy of the
    /// original payload and recompresses it into a valid gzip member. The
    /// receiver stays immutable; nothing on disk is touched here.
    pub fn serialize_with_patches(&self, patches: &[BytePatch]) -> Result<Vec<u8>, EditError> {
        let mut payload = self.payload.clone();
        apply_patches(&mut payload, patches)?;
        Ok(compress_gzip(&payload))
    }

    /// Plans the checked patches for `edits` against one town record.
    /// Garrison edits require exact 44/11, a verified town span, and a
    /// garrison not owned by a garrisoned hero; everything else in the town
    /// record (name, buildings, mage guild, hero bytes, unknown tail) stays
    /// byte-identical.
    pub fn plan_town_edits(
        &self,
        town_id: usize,
        edits: &[TownEdit],
    ) -> Result<Vec<BytePatch>, EditError> {
        if !self.format.army_edit_supported() {
            return Err(EditError::UnsupportedFormat {
                family: self.format.family_title(),
                major: self.format.major,
                minor: self.format.minor,
            });
        }
        let town = self
            .towns
            .get(town_id)
            .ok_or(EditError::TownNotFound { id: town_id })?;
        if town.confidence != crate::domain::TownConfidence::High {
            return Err(EditError::TownNotEditable { id: town_id });
        }
        let base = town.span.start;
        let record = &self.payload[town.span.clone()];
        let mut patches = Vec::new();
        let mut built_target: Option<u64> = None;
        let mut guild_targets: [Option<Vec<u8>>; crate::town_state::GUILD_LEVELS] =
            Default::default();
        for edit in edits {
            match edit {
                TownEdit::SetBuildings { built } => built_target = Some(*built),
                TownEdit::SetGuildSpells { level, spells } => {
                    let index = usize::from(*level).wrapping_sub(1);
                    let slot = guild_targets
                        .get_mut(index)
                        .ok_or(EditError::InvalidValue {
                            field: "mage guild",
                            reason: "guild levels are 1-5".into(),
                        })?;
                    *slot = Some(spells.clone());
                }
                TownEdit::SetGarrisonSlot {
                    slot,
                    creature_id,
                    count,
                } => {
                    if town.garrison_hero.is_some() {
                        return Err(EditError::InvalidValue {
                            field: "garrison",
                            reason: "this garrison belongs to a garrisoned hero; edit that \
                                     hero's army instead"
                                .into(),
                        });
                    }
                    plan_stack_slot(
                        &mut patches,
                        base,
                        crate::definitions::towns::GARRISON_IDS,
                        crate::definitions::towns::GARRISON_COUNTS,
                        record,
                        &town.garrison,
                        *slot,
                        *creature_id,
                        *count,
                        "garrison slot",
                    )?;
                }
            }
        }
        if built_target.is_some() || guild_targets.iter().any(Option::is_some) {
            plan_town_state(
                &mut patches,
                base,
                record,
                town,
                built_target,
                &guild_targets,
            )?;
        }
        resolve_patch_conflicts(patches)
    }
}

/// Plans the building-mask and Mage Guild patches of one town (H3F-022):
/// validates the desired built mask and spell lists against the stored
/// state, derives the coupled fields, and emits one checked patch per
/// changed field. Fields the model does not cover are never written.
fn plan_town_state(
    patches: &mut Vec<BytePatch>,
    base: usize,
    record: &[u8],
    town: &crate::domain::TownView,
    built_target: Option<u64>,
    guild_targets: &[Option<Vec<u8>>; crate::town_state::GUILD_LEVELS],
) -> Result<(), EditError> {
    use crate::town_state::TownState;
    let unavailable = || EditError::InvalidValue {
        field: "town state",
        reason: "building and Mage Guild fields are not available for this record".into(),
    };
    let state = town.state.as_ref().ok_or_else(unavailable)?;
    let tail = TownState::tail_offset(record).ok_or_else(unavailable)?;
    if let Some(target) = built_target {
        state.validate_built_target(target)?;
        // A building change rewrites the guild counts, so the guild fields
        // must follow the verified representation as well.
        if target != state.built
            && let Some(issue) = state.guild_issue()
        {
            return Err(EditError::InvalidValue {
                field: "buildings",
                reason: issue,
            });
        }
    }
    for (level, spells) in guild_targets.iter().enumerate() {
        if let Some(spells) = spells {
            state.validate_guild_spells(level, spells)?;
        }
    }
    let next = state.with_changes(built_target, guild_targets);
    if next.mage_guild_level != state.mage_guild_level {
        patches.push(BytePatch::new(
            base + crate::definitions::towns::MAGE_GUILD_LEVEL,
            [state.mage_guild_level],
            [next.mage_guild_level],
            "mage guild level",
        )?);
    }
    for (offset, old, new, label) in state.diff(&next) {
        patches.push(BytePatch::new(base + tail + offset, old, new, label)?);
    }
    Ok(())
}

/// Adds a single-byte patch when `value` differs from the stored byte.
fn push_byte(
    patches: &mut Vec<BytePatch>,
    base: usize,
    offset: usize,
    record: &[u8],
    value: u8,
    range: std::ops::RangeInclusive<u8>,
    field: &'static str,
) -> Result<(), EditError> {
    if !range.contains(&value) {
        return Err(EditError::InvalidValue {
            field,
            reason: format!("{value} is outside the supported range {range:?}"),
        });
    }
    push_scalar(patches, base, offset, [value], record, field)
}

/// Adds a patch for a little-endian scalar field, skipping no-ops.
fn push_scalar<const N: usize>(
    patches: &mut Vec<BytePatch>,
    base: usize,
    offset: usize,
    value: [u8; N],
    record: &[u8],
    reason: &str,
) -> Result<(), EditError> {
    let current: [u8; N] = record[offset..offset + N]
        .try_into()
        .expect("located records contain every edit offset");
    if current == value {
        return Ok(()); // No-op: nothing to patch.
    }
    patches.push(BytePatch::new(base + offset, current, value, reason)?);
    Ok(())
}

/// Validates and plans the replacement of one creature-stack slot, shared by
/// hero armies and town garrisons: identical word layout (IDs then counts,
/// u32 little-endian) and h3sed `common/army.py:serialize` semantics -
/// unknown occupied stacks stay read-only, new empty slots write the
/// `FF` ID + zero count form, and untouched/empty original encodings are
/// preserved verbatim.
#[allow(clippy::too_many_arguments)]
fn plan_stack_slot(
    patches: &mut Vec<BytePatch>,
    base: usize,
    ids_offset: usize,
    counts_offset: usize,
    record: &[u8],
    original: &[crate::domain::ArmySlot],
    slot: u8,
    creature_id: Option<u32>,
    count: u32,
    label: &str,
) -> Result<(), EditError> {
    let original = original
        .get(slot as usize)
        .ok_or_else(|| EditError::InvalidValue {
            field: "army slot",
            reason: "slot must be between 1 and 7".into(),
        })?;
    if !original.is_supported() {
        return Err(EditError::InvalidValue {
            field: "army slot",
            reason: "unrecognized stack is read-only".into(),
        });
    }
    match creature_id {
        Some(id) if crate::creatures::name(id).is_none() || count == 0 => {
            return Err(EditError::InvalidValue {
                field: "army stack",
                reason: "choose a supported creature and a count from 1 to 4294967295".into(),
            });
        }
        None if count != 0 => {
            return Err(EditError::InvalidValue {
                field: "army stack",
                reason: "an empty stack must have count zero".into(),
            });
        }
        _ => {}
    }
    // Preserve every original empty encoding on clear/no-op/revert.
    if creature_id.is_none() && original.is_empty() {
        return Ok(());
    }
    let reason = format!("{label} {}", slot + 1);
    push_scalar(
        patches,
        base,
        ids_offset + slot as usize * 4,
        creature_id.unwrap_or(u32::MAX).to_le_bytes(),
        record,
        &reason,
    )?;
    push_scalar(
        patches,
        base,
        counts_offset + slot as usize * 4,
        count.to_le_bytes(),
        record,
        &reason,
    )?;
    Ok(())
}

/// Plans skill-table patches for one absolute ordered (skill ID, mastery)
/// list - h3sed `hero/skills.py:serialize` semantics made conservative.
/// Listed skills get dense one-based order bytes at 1092 and their mastery
/// byte at 182; base-record skills missing from the list are cleared (the
/// removal); bytes of entries the parser does not recognize, and the
/// obsolete legacy region at 212..238, are never touched. The stored count
/// (low byte of the u32 at 43) caps at 28 even though 30 skill IDs exist in
/// 44/11 (h3sed DATA_RANGES "skills").
fn plan_skills(
    patches: &mut Vec<BytePatch>,
    base: usize,
    record: &[u8],
    skills: &[(u8, u8)],
) -> Result<(), EditError> {
    use crate::definitions::hero::{
        RECORD_LEN, SKILL_COUNT, SKILL_COUNT_MAX, SKILL_LEVEL_MAX, SKILL_LEVELS, SKILL_SLOTS_HOT_A,
    };
    use crate::definitions::names::SKILL_NAMES;
    if record.len() != RECORD_LEN {
        return Err(EditError::InvalidValue {
            field: "skills",
            reason: "the 1122-byte HotA record is required for skill order bytes".into(),
        });
    }
    if skills.len() > SKILL_COUNT_MAX as usize {
        return Err(EditError::InvalidValue {
            field: "skills",
            reason: format!("at most {SKILL_COUNT_MAX} skills fit the stored count"),
        });
    }
    let mut seen = Vec::with_capacity(skills.len());
    for &(skill_id, mastery) in skills {
        if (skill_id as usize) >= SKILL_NAMES.len() {
            return Err(EditError::InvalidValue {
                field: "skills",
                reason: format!("skill ID {skill_id} is not defined for 44/11"),
            });
        }
        if !(1..=SKILL_LEVEL_MAX).contains(&mastery) {
            return Err(EditError::InvalidValue {
                field: "skills",
                reason: format!("mastery {mastery} is outside 1-{SKILL_LEVEL_MAX}"),
            });
        }
        if seen.contains(&skill_id) {
            return Err(EditError::InvalidValue {
                field: "skills",
                reason: format!("duplicate skill {skill_id}"),
            });
        }
        seen.push(skill_id);
    }

    let base_count = record[SKILL_COUNT] as usize;
    let listed = |skill_id: u8| skills.iter().any(|&(id, _)| id == skill_id);
    // Entries absent from the new list: clear the ones the game would list
    // today (mastery, slot and base count all agree); leave unrecognized
    // bytes (zero slot, zero mastery, or a slot beyond the base count)
    // untouched. The new count is the dense list length, which never exceeds
    // the base count unless skills were added - so previously-unreadable
    // entries stay unreadable in every normal case.
    for skill_id in 0..SKILL_NAMES.len() as u8 {
        let mastery_at = SKILL_LEVELS + skill_id as usize;
        let slot_at = SKILL_SLOTS_HOT_A + skill_id as usize;
        let (base_mastery, base_slot) = (record[mastery_at], record[slot_at]);
        if listed(skill_id) {
            continue;
        }
        let base_present = base_mastery != 0 && base_slot != 0 && base_slot as usize <= base_count;
        if base_present {
            push_scalar(patches, base, mastery_at, [0u8], record, "skill removed")?;
            push_scalar(patches, base, slot_at, [0u8], record, "skill removed")?;
        }
    }
    push_scalar(
        patches,
        base,
        SKILL_COUNT,
        [skills.len() as u8],
        record,
        "skill count",
    )?;
    for (index, &(skill_id, mastery)) in skills.iter().enumerate() {
        let name = crate::definitions::names::skill_name(skill_id as usize)
            .map(str::to_string)
            .unwrap_or_default();
        let reason = format!("skill {skill_id} ({name})");
        push_scalar(
            patches,
            base,
            SKILL_LEVELS + skill_id as usize,
            [mastery],
            record,
            &reason,
        )?;
        push_scalar(
            patches,
            base,
            SKILL_SLOTS_HOT_A + skill_id as usize,
            [(index + 1) as u8],
            record,
            &reason,
        )?;
    }
    Ok(())
}

/// Plans the 8-byte replacement of one backpack slot, h3sed
/// `hero/inventory.py:serialize` encodings: artifacts write
/// `ID 00 00 00 FF FF FF FF`, scrolls the combined
/// `01 00 00 00 SS 00 00 00`, removals h3sed's canonical blank
/// `FF FF FF FF 00 00 00 00`. Only this slot's eight bytes are patched;
/// untouched slots (either blank encoding), unknown entries, equipment and
/// reservation bytes are never modified.
fn plan_backpack_slot(
    patches: &mut Vec<BytePatch>,
    base: usize,
    record: &[u8],
    hero: &crate::domain::HeroView,
    slot: u8,
    item: BackpackEdit,
) -> Result<(), EditError> {
    use crate::artifacts::{BACKPACK_SLOTS, SCROLL_ARTIFACT_ID, SPELLBOOK_ARTIFACT_ID};
    use crate::definitions::hero::INVENTORY;
    if slot as usize >= BACKPACK_SLOTS {
        return Err(EditError::InvalidValue {
            field: "backpack slot",
            reason: format!("slot must be between 1 and {BACKPACK_SLOTS}"),
        });
    }
    if hero.backpack[slot as usize] == crate::domain::BackpackItem::Unknown {
        return Err(EditError::InvalidValue {
            field: "backpack slot",
            reason: "unknown entry is preserved read-only".into(),
        });
    }
    if matches!(hero.backpack[slot as usize], crate::BackpackItem::Artifact(id) if !crate::artifacts::writable(id))
    {
        return Err(EditError::InvalidValue {
            field: "backpack slot",
            reason: "unsupported artifact is preserved read-only".into(),
        });
    }
    let desired = match item {
        BackpackEdit::Empty => crate::BackpackItem::Empty,
        BackpackEdit::Artifact(id) => crate::BackpackItem::Artifact(id),
        BackpackEdit::Scroll { spell_id } => crate::BackpackItem::Scroll { spell_id },
    };
    let replacement: [u8; 8] = match item {
        BackpackEdit::Empty => [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0],
        BackpackEdit::Artifact(artifact_id) => {
            if artifact_id == SCROLL_ARTIFACT_ID {
                return Err(EditError::InvalidValue {
                    field: "backpack artifact",
                    reason: "scrolls are added as scrolls, not by their item ID".into(),
                });
            }
            if artifact_id == SPELLBOOK_ARTIFACT_ID {
                return Err(EditError::InvalidValue {
                    field: "backpack artifact",
                    reason: "the spellbook is indistinguishable from an empty slot here".into(),
                });
            }
            if !crate::artifacts::writable(artifact_id) {
                return Err(EditError::InvalidValue {
                    field: "backpack artifact",
                    reason: format!("artifact ID {artifact_id} is not verified for 44/11"),
                });
            }
            let mut bytes = [0u8; 8];
            bytes[..4].copy_from_slice(&artifact_id.to_le_bytes());
            bytes[4..].fill(0xFF);
            bytes
        }
        BackpackEdit::Scroll { spell_id } => {
            if crate::definitions::names::spell_name(spell_id as usize).is_none() {
                return Err(EditError::InvalidValue {
                    field: "backpack scroll",
                    reason: format!("spell ID {spell_id} has no verified name"),
                });
            }
            let mut bytes = [0u8; 8];
            bytes[..4].copy_from_slice(&SCROLL_ARTIFACT_ID.to_le_bytes());
            bytes[4] = spell_id;
            bytes
        }
    };
    if hero.backpack[slot as usize] == desired {
        return Ok(());
    }
    push_scalar(
        patches,
        base,
        INVENTORY + slot as usize * 8,
        replacement,
        record,
        &format!("backpack slot {}", slot + 1),
    )?;
    Ok(())
}

/// Plans every equipment-slot edit of one batch as a single desired state.
///
/// The slot words, the stored primary attributes (which include equipped
/// bonuses), artifact-granted spell availability and the combination
/// reservation counters at 1047-1055 are all derived from the complete
/// desired equipment, so several slot changes in one batch never conflict.
/// Only bytes whose modeled value changes are patched. Records with
/// unmodeled occupants or unexplained reservation bytes stay blocked.
fn plan_equipment(
    patches: &mut Vec<BytePatch>,
    base: usize,
    record: &[u8],
    hero: &crate::domain::HeroView,
    edits: &[(u8, EquipmentEdit)],
) -> Result<(), EditError> {
    use crate::artifacts::{
        self, BANNABLE_SPELLS, EquipmentSlotKind, SCROLL_ARTIFACT_ID, SPELLBOOK_ARTIFACT_ID,
    };
    use crate::definitions::hero::{
        ATTACK, DEFENSE, EQUIPMENT, EQUIPMENT_SLOT_LEN, EQUIPMENT_SLOTS, KNOWLEDGE, RESERVED,
        SPELL_FLAG_COUNT, SPELL_POWER, SPELLS_AVAILABLE, SPELLS_LEARNED,
    };
    use crate::definitions::names::{EQUIPMENT_SLOT_LABELS, spell_name};
    use crate::domain::EquipmentItem;
    if edits.is_empty() {
        return Ok(());
    }
    for &(slot, _) in edits {
        if artifacts::wear_slot_kind(slot).is_none() {
            return Err(EditError::InvalidValue {
                field: "equipment slot",
                reason:
                    "only wear slots are editable; war machines and the spellbook stay read-only"
                        .into(),
            });
        }
    }
    if let Some(reason) = hero.equipment_block_reason() {
        return Err(EditError::InvalidValue {
            field: "equipment",
            reason,
        });
    }

    // Base and desired artifact per wear slot (gating guarantees every wear
    // occupant is a modeled artifact of the slot's own kind).
    let mut current: [Option<u32>; EQUIPMENT_SLOTS] = [None; EQUIPMENT_SLOTS];
    for (index, entry) in current.iter_mut().enumerate() {
        if artifacts::wear_slot_kind(index as u8).is_some()
            && let EquipmentItem::Artifact(id) = hero.equipment_items[index]
        {
            *entry = Some(id);
        }
    }
    let mut desired = current;
    for &(slot, item) in edits {
        let wear_kind = artifacts::wear_slot_kind(slot).expect("checked above");
        desired[slot as usize] = match item {
            EquipmentEdit::Empty => None,
            EquipmentEdit::Artifact(id) => {
                if id == SCROLL_ARTIFACT_ID
                    || id == SPELLBOOK_ARTIFACT_ID
                    || !artifacts::writable(id)
                {
                    return Err(EditError::InvalidValue {
                        field: "equipment artifact",
                        reason: format!("artifact ID {id} is not verified for equipping in 44/11"),
                    });
                }
                let name = artifacts::name(id).unwrap_or("?");
                match artifacts::slot_kind(id) {
                    Some(kind) if kind == wear_kind => {}
                    Some(kind) => {
                        return Err(EditError::InvalidValue {
                            field: "equipment artifact",
                            reason: format!("{name} fits the {} slot, not this one", kind.label()),
                        });
                    }
                    None => {
                        return Err(EditError::InvalidValue {
                            field: "equipment artifact",
                            reason: format!("{name} cannot be equipped (inventory-only artifact)"),
                        });
                    }
                }
                Some(id)
            }
        };
    }

    // Slot capacity: worn items plus combination reservations per kind.
    let reserved_old = artifacts::reservation_counts(current.iter().flatten().copied());
    let reserved_new = artifacts::reservation_counts(desired.iter().flatten().copied());
    for kind in EquipmentSlotKind::ALL {
        let occupied = (0..EQUIPMENT_SLOTS)
            .filter(|&index| {
                desired[index].is_some() && artifacts::wear_slot_kind(index as u8) == Some(kind)
            })
            .count();
        let reserved = reserved_new[kind.reservation_index()] as usize;
        if reserved > 0 && occupied + reserved > kind.capacity() as usize {
            let combos: Vec<&str> = desired
                .iter()
                .flatten()
                .filter(|&&id| artifacts::reserved_slots(id).contains(&kind))
                .map(|&id| artifacts::name(id).unwrap_or("?"))
                .collect();
            let worn: Vec<String> = (0..EQUIPMENT_SLOTS)
                .filter(|&index| artifacts::wear_slot_kind(index as u8) == Some(kind))
                .filter_map(|index| {
                    desired[index].map(|id| {
                        format!(
                            "{} ({})",
                            artifacts::name(id).unwrap_or("?"),
                            EQUIPMENT_SLOT_LABELS[index]
                        )
                    })
                })
                .collect();
            let capacity = kind.capacity() as usize;
            return Err(EditError::InvalidValue {
                field: "equipment",
                reason: format!(
                    "{} needs {reserved} free {} slot(s), but only {} of {capacity} would be free. Unequip first: {}.",
                    combos.join(" and "),
                    kind.label(),
                    capacity.saturating_sub(occupied),
                    worn.join(", "),
                ),
            });
        }
    }

    // Slot words: untouched and unchanged slots keep their original bytes
    // (the game writes several blank encodings).
    for index in 0..EQUIPMENT_SLOTS {
        if desired[index] == current[index] {
            continue;
        }
        let at = EQUIPMENT + index * EQUIPMENT_SLOT_LEN;
        let (bytes, label) = match desired[index] {
            Some(id) => {
                let mut bytes = [0u8; 8];
                bytes[..4].copy_from_slice(&id.to_le_bytes());
                (
                    bytes,
                    format!("equip {}", artifacts::name(id).unwrap_or("?")),
                )
            }
            None => (
                [0xFF; 8],
                format!(
                    "unequip {}",
                    current[index].and_then(artifacts::name).unwrap_or("item")
                ),
            ),
        };
        push_scalar(patches, base, at, bytes, record, &label)?;
    }

    // Reservation counters: the base bytes equal the modeled counters
    // (gating), so only counters whose value changes are written.
    for kind in EquipmentSlotKind::ALL {
        let index = kind.reservation_index();
        if reserved_new[index] != reserved_old[index] {
            push_scalar(
                patches,
                base,
                RESERVED + index,
                [reserved_new[index]],
                record,
                &format!("combination reservation ({} slots)", kind.label()),
            )?;
        }
    }

    // Attribute coupling: stored attributes include equipped bonuses, so
    // they move by the bonus delta of the whole equipment change.
    let bonus_of = |ids: &[Option<u32>; EQUIPMENT_SLOTS]| -> [i32; 4] {
        let mut total = [0i32; 4];
        for id in ids.iter().flatten() {
            if let Some(bonus) = artifacts::stat_bonus(*id) {
                for (index, value) in bonus.iter().enumerate() {
                    total[index] += *value as i32;
                }
            }
        }
        total
    };
    let old_bonus = bonus_of(&current);
    let new_bonus = bonus_of(&desired);
    for (offset, index) in [(ATTACK, 0), (DEFENSE, 1), (SPELL_POWER, 2), (KNOWLEDGE, 3)] {
        let delta = new_bonus[index] - old_bonus[index];
        if delta == 0 {
            continue;
        }
        let value = (record[offset] as i32 + delta).rem_euclid(256) as u8;
        push_scalar(
            patches,
            base,
            offset,
            [value],
            record,
            "equipment attribute bonus",
        )?;
    }

    // Spell coupling: only spells entering or leaving the modeled grant set
    // are touched; learned flags and unrelated deviations are never written.
    let grants = |ids: &[Option<u32>; EQUIPMENT_SLOTS]| -> Vec<u8> {
        ids.iter()
            .flatten()
            .flat_map(|&id| artifacts::spell_grants(id).iter().copied())
            .collect()
    };
    let grants_old = grants(&current);
    let grants_new = grants(&desired);
    for spell_id in 0..SPELL_FLAG_COUNT as u8 {
        let in_book = hero.has_spellbook && record[SPELLS_LEARNED + spell_id as usize] == 1;
        let added = grants_new.contains(&spell_id) && !grants_old.contains(&spell_id);
        let removed = grants_old.contains(&spell_id) && !grants_new.contains(&spell_id);
        // New grant: set the flag, except bannable spells the save keeps at
        // zero (map ban detection, h3sed's conditional). Withdrawn grant:
        // clear it unless the spell is also in the book.
        let available_new = if added && !BANNABLE_SPELLS.contains(&spell_id) {
            true
        } else if removed && !in_book {
            false
        } else {
            continue;
        };
        let reason = match spell_name(spell_id as usize) {
            Some(name) => format!("spell {spell_id} ({name}) availability"),
            None => format!("spell {spell_id} availability"),
        };
        push_scalar(
            patches,
            base,
            SPELLS_AVAILABLE + spell_id as usize,
            [available_new as u8],
            record,
            &reason,
        )?;
    }
    Ok(())
}

/// Final pass over one planned batch: byte-identical duplicate patches (two
/// edits computing the same change, e.g. learning a spell an equipped artifact
/// also grants) collapse into one; any remaining overlap means two edits
/// want different values in the same bytes, which is reported as an explicit
/// conflict instead of failing later at apply time.
fn resolve_patch_conflicts(mut patches: Vec<BytePatch>) -> Result<Vec<BytePatch>, EditError> {
    patches.sort_by_key(|patch| patch.offset);
    let mut merged: Vec<BytePatch> = Vec::with_capacity(patches.len());
    for patch in patches {
        let Some(last) = merged.last_mut() else {
            merged.push(patch);
            continue;
        };
        if patch.offset >= last.end() {
            merged.push(patch);
            continue;
        }
        if last.expected == patch.expected && last.replacement == patch.replacement {
            continue;
        }
        return Err(EditError::InvalidValue {
            field: "edit batch",
            reason: format!(
                "conflicting edits: '{}' and '{}' write the same bytes with different values; \
                 revert one of them",
                last.reason, patch.reason
            ),
        });
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::hero::{
        ARMY_COUNTS, ARMY_IDS, EQUIPMENT, EQUIPMENT_SLOT_LEN, EQUIPMENT_SLOTS, INVENTORY,
        INVENTORY_ENTRIES, MOVEMENT_LEFT, MOVEMENT_TOTAL, NAME, NAME_LEN, OWNER, RECORD_LEN,
        RESERVED, RESERVED_LEN, SKILL_COUNT, SKILL_LEVELS, SKILL_SLOTS_HOT_A, SPELL_FLAG_COUNT,
    };
    use crate::{Compatibility, SaveDocument};

    /// Structurally valid 1122-byte HotA record, mirroring the locator tests.
    fn craft_record() -> Vec<u8> {
        let mut record = vec![0x55u8; RECORD_LEN];
        record[OWNER] = 4;
        record[MOVEMENT_TOTAL..MOVEMENT_TOTAL + 4].copy_from_slice(&1500u32.to_le_bytes());
        record[MOVEMENT_LEFT..MOVEMENT_LEFT + 4].copy_from_slice(&1500u32.to_le_bytes());
        record[EXPERIENCE..EXPERIENCE + 4].copy_from_slice(&1064u32.to_le_bytes());
        record[SKILL_COUNT] = 3;
        record[SKILL_COUNT + 1..SKILL_COUNT + 4].fill(0);
        record[SPELL_POINTS..SPELL_POINTS + 2].copy_from_slice(&73u16.to_le_bytes());
        record[LEVEL] = 2;
        record[ARMY_IDS..ARMY_IDS + 28].fill(0);
        record[ARMY_COUNTS..ARMY_COUNTS + 28].fill(0);
        let name = b"Aiden";
        record[NAME..NAME + name.len()].copy_from_slice(name);
        record[NAME + name.len()..NAME + NAME_LEN].fill(0);
        record[SKILL_LEVELS..SKILL_LEVELS + 30].fill(0);
        record[ATTACK] = 3;
        record[DEFENSE] = 7;
        record[SPELL_POWER] = 6;
        record[KNOWLEDGE] = 6;
        record[SPELLS_LEARNED..SPELLS_LEARNED + SPELL_FLAG_COUNT].fill(0);
        record[SPELLS_AVAILABLE..SPELLS_AVAILABLE + SPELL_FLAG_COUNT].fill(0);
        for slot in 0..EQUIPMENT_SLOTS {
            let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
            record[at..at + 4].fill(0xFF);
            record[at + 4..at + 8].fill(0);
        }
        for entry in 0..INVENTORY_ENTRIES {
            record[INVENTORY + entry * 8..INVENTORY + entry * 8 + 8].fill(0xFF);
        }
        // Combination-artifact reservation bytes: zero (editable state).
        record[RESERVED..RESERVED + RESERVED_LEN].fill(0);
        record[SKILL_SLOTS_HOT_A..SKILL_SLOTS_HOT_A + 30].fill(0);
        record
    }

    fn embed(record: &[u8], at: usize) -> Vec<u8> {
        let mut payload = vec![0u8; 40_000];
        payload[..5].copy_from_slice(b"H3SVG");
        payload[8..12].copy_from_slice(&44u32.to_le_bytes());
        payload[12..16].copy_from_slice(&11u32.to_le_bytes());
        payload[at..at + record.len()].copy_from_slice(record);
        payload
    }

    fn open_with_record(record: &[u8], at: usize) -> SaveDocument {
        SaveDocument::open(crate::patch::compress_gzip(&embed(record, at))).expect("opens")
    }

    #[test]
    fn backpack_operations_preserve_every_other_byte() {
        let mut record = craft_record();
        record[INVENTORY + 8..INVENTORY + 16].copy_from_slice(&[254, 0, 0, 0, 12, 0, 0, 0]);
        record[INVENTORY + 20..INVENTORY + 24].fill(0);
        let mut doc = open_with_record(&record, 30_500);
        for item in [
            BackpackEdit::Artifact(7),
            BackpackEdit::Artifact(126),
            BackpackEdit::Scroll { spell_id: 15 },
            BackpackEdit::Scroll { spell_id: 0 },
            BackpackEdit::Empty,
        ] {
            let plan = doc
                .plan_hero_edits(0, &[HeroEdit::SetBackpackSlot { slot: 0, item }])
                .unwrap();
            assert_eq!(plan.len(), 1);
            assert_eq!(plan[0].offset, 30_500 + INVENTORY);
            assert_eq!(plan[0].expected.len(), 8);
            let next = SaveDocument::open(doc.serialize_with_patches(&plan).unwrap()).unwrap();
            assert_eq!(
                &doc.payload()[..30_500 + INVENTORY],
                &next.payload()[..30_500 + INVENTORY]
            );
            assert_eq!(
                &doc.payload()[30_500 + INVENTORY + 8..],
                &next.payload()[30_500 + INVENTORY + 8..]
            );
            if let BackpackEdit::Scroll { spell_id } = item {
                assert_eq!(next.heroes[0].inventory[0], [1, 0, 0, 0, spell_id, 0, 0, 0]);
            }
            doc = next;
        }
        assert_eq!(doc.heroes[0].inventory[0], [255, 255, 255, 255, 0, 0, 0, 0]);
        for (slot, item) in [
            (1, BackpackEdit::Empty),
            (0, BackpackEdit::Artifact(144)),
            (0, BackpackEdit::Artifact(166)),
            (0, BackpackEdit::Artifact(3)),
            (0, BackpackEdit::Artifact(0)),
            (0, BackpackEdit::Artifact(1)),
            (0, BackpackEdit::Scroll { spell_id: 255 }),
            (64, BackpackEdit::Artifact(7)),
        ] {
            assert!(
                doc.plan_hero_edits(0, &[HeroEdit::SetBackpackSlot { slot, item }])
                    .is_err()
            );
        }
        assert!(
            doc.plan_hero_edits(
                0,
                &[HeroEdit::SetBackpackSlot {
                    slot: 2,
                    item: BackpackEdit::Empty
                }]
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn backpack_full_and_decode_safety() {
        use crate::BackpackItem;
        let mut record = craft_record();
        for slot in 0..64 {
            record[INVENTORY + slot * 8..INVENTORY + slot * 8 + 8]
                .copy_from_slice(&[7, 0, 0, 0, 255, 255, 255, 255]);
        }
        let doc = open_with_record(&record, 30_500);
        assert!(!doc.heroes[0].backpack.contains(&BackpackItem::Empty));
        assert!(
            doc.plan_hero_edits(
                0,
                &[HeroEdit::SetBackpackSlot {
                    slot: 64,
                    item: BackpackEdit::Artifact(7)
                }]
            )
            .is_err()
        );
        assert_eq!(
            BackpackItem::decode(&[0, 0, 0, 0, 255, 255, 255, 255]),
            BackpackItem::Unknown
        );
        assert_eq!(
            BackpackItem::decode(&[7, 0, 0, 0, 42, 0, 0, 0]),
            BackpackItem::Unknown
        );
        assert_eq!(
            BackpackItem::decode(&[1, 0, 0, 0, 15, 1, 0, 0]),
            BackpackItem::Unknown
        );
        assert_eq!(
            BackpackItem::decode(&[255, 255, 255, 255, 42, 0, 0, 0]),
            BackpackItem::Empty
        );
    }

    #[test]
    fn army_slots_roundtrip_and_only_changed_words() {
        let mut record = craft_record();
        // Mix both blank encodings, including FF count, and an unknown occupied slot.
        record[ARMY_IDS + 4..ARMY_IDS + 8].fill(0xff);
        record[ARMY_COUNTS + 4..ARMY_COUNTS + 8].fill(0xff);
        record[ARMY_IDS + 24..ARMY_IDS + 28].copy_from_slice(&999u32.to_le_bytes());
        record[ARMY_COUNTS + 24..ARMY_COUNTS + 28].copy_from_slice(&5u32.to_le_bytes());
        let doc = open_with_record(&record, 30_500);
        for slot in 0..6 {
            let plan = doc
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetArmySlot {
                        slot,
                        creature_id: Some(199),
                        count: u32::MAX,
                    }],
                )
                .unwrap();
            let reopened = SaveDocument::open(doc.serialize_with_patches(&plan).unwrap()).unwrap();
            assert_eq!(reopened.heroes[0].army[slot as usize].creature_id, 199);
            assert_eq!(reopened.heroes[0].army[slot as usize].count, u32::MAX);
            for (index, (before, after)) in doc.payload().iter().zip(reopened.payload()).enumerate()
            {
                if before != after {
                    assert!(
                        (30_500 + ARMY_IDS + slot as usize * 4
                            ..30_500 + ARMY_IDS + slot as usize * 4 + 4)
                            .contains(&index)
                            || (30_500 + ARMY_COUNTS + slot as usize * 4
                                ..30_500 + ARMY_COUNTS + slot as usize * 4 + 4)
                                .contains(&index)
                    );
                }
            }
            let clear = reopened
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetArmySlot {
                        slot,
                        creature_id: None,
                        count: 0,
                    }],
                )
                .unwrap();
            let cleared =
                SaveDocument::open(reopened.serialize_with_patches(&clear).unwrap()).unwrap();
            assert_eq!(cleared.heroes[0].army[slot as usize].creature_id, u32::MAX);
            assert_eq!(cleared.heroes[0].army[slot as usize].count, 0);
        }
        for slot in [0, 1] {
            assert!(
                doc.plan_hero_edits(
                    0,
                    &[HeroEdit::SetArmySlot {
                        slot,
                        creature_id: None,
                        count: 0
                    }]
                )
                .unwrap()
                .is_empty()
            );
        }
        assert!(
            doc.plan_hero_edits(
                0,
                &[HeroEdit::SetArmySlot {
                    slot: 6,
                    creature_id: Some(0),
                    count: 1
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn army_validation_and_seventh_slot() {
        let doc = open_with_record(&craft_record(), 30_500);
        for edit in [
            HeroEdit::SetArmySlot {
                slot: 7,
                creature_id: Some(0),
                count: 1,
            },
            HeroEdit::SetArmySlot {
                slot: 0,
                creature_id: Some(145),
                count: 1,
            },
            HeroEdit::SetArmySlot {
                slot: 0,
                creature_id: Some(u32::MAX),
                count: 1,
            },
            HeroEdit::SetArmySlot {
                slot: 0,
                creature_id: Some(0),
                count: 0,
            },
            HeroEdit::SetArmySlot {
                slot: 0,
                creature_id: None,
                count: 1,
            },
        ] {
            assert!(matches!(
                doc.plan_hero_edits(0, &[edit]),
                Err(EditError::InvalidValue { .. })
            ));
        }
        let edit = HeroEdit::SetArmySlot {
            slot: 6,
            creature_id: Some(0),
            count: 1,
        };
        let plan = doc.plan_hero_edits(0, std::slice::from_ref(&edit)).unwrap();
        assert_eq!(
            plan.len(),
            1,
            "Pikeman ID zero already stored; only count changes"
        );
        let reopened = SaveDocument::open(doc.serialize_with_patches(&plan).unwrap()).unwrap();
        assert!(!reopened.heroes[0].army[6].is_empty());
        assert!(
            reopened
                .plan_hero_edits(0, std::slice::from_ref(&edit))
                .unwrap()
                .is_empty()
        );
        for minor in [5, 10, 12] {
            let mut unsupported = doc.clone();
            unsupported.format.minor = minor;
            assert!(matches!(
                unsupported.plan_hero_edits(0, std::slice::from_ref(&edit)),
                Err(EditError::UnsupportedFormat { .. })
            ));
        }
        let mut low = doc.clone();
        low.heroes[0].confidence = HeroConfidence::Low;
        assert!(matches!(
            low.plan_hero_edits(0, std::slice::from_ref(&edit)),
            Err(EditError::RecordNotEditable { .. })
        ));
        assert_eq!(crate::creatures::name(0), Some("Pikeman"));
        assert_eq!(crate::creatures::name(153), Some("Nymph"));
        assert_eq!(crate::creatures::name(185), Some("Juggernaut"));
        assert_eq!(crate::creatures::name(199), Some("Jotunn Warlord"));
        let ids: std::collections::BTreeSet<_> = crate::creatures::HOTA_44_11
            .iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids.len(), crate::creatures::HOTA_44_11.len());
    }

    #[test]
    fn no_op_edits_plan_no_patches() {
        let document = open_with_record(&craft_record(), 30_500);
        let plan = document
            .plan_hero_edits(
                0,
                &[
                    HeroEdit::SetAttack(3),
                    HeroEdit::SetExperience(1064),
                    HeroEdit::SetSpellPoints(73),
                    HeroEdit::SetLevel(2),
                ],
            )
            .expect("plans");
        assert!(plan.is_empty());
    }

    #[test]
    fn sets_primary_attributes_and_scalars() {
        let document = open_with_record(&craft_record(), 30_500);
        let plan = document
            .plan_hero_edits(
                0,
                &[
                    HeroEdit::SetAttack(12),
                    HeroEdit::SetKnowledge(9),
                    HeroEdit::SetExperience(2_000),
                    HeroEdit::SetLevel(3),
                    HeroEdit::SetSpellPoints(80),
                ],
            )
            .expect("plans");
        assert_eq!(plan.len(), 5);
        let bytes = document.serialize_with_patches(&plan).expect("serializes");
        let reopened = SaveDocument::open(bytes).expect("reopens");
        let hero = &reopened.heroes[0];
        assert_eq!(hero.primary.attack, 12);
        assert_eq!(hero.primary.knowledge, 9);
        assert_eq!(hero.experience, 2_000);
        assert_eq!(hero.level, 3);
        assert_eq!(hero.spell_points, 80);
        // Defence and spell power were not edited.
        assert_eq!(hero.primary.defense, 7);
        assert_eq!(hero.primary.spell_power, 6);
    }

    #[test]
    fn rejects_out_of_range_values() {
        let document = open_with_record(&craft_record(), 30_500);
        for edit in [
            HeroEdit::SetAttack(100),
            HeroEdit::SetSpellPower(0),
            HeroEdit::SetSpellPower(100),
            HeroEdit::SetLevel(0),
            HeroEdit::SetLevel(75),
            HeroEdit::SetSpellLearned {
                spell_id: SPELL_FLAG_COUNT as u8,
                learned: true,
            },
        ] {
            assert!(
                matches!(
                    document.plan_hero_edits(0, std::slice::from_ref(&edit)),
                    Err(EditError::InvalidValue { .. })
                ),
                "{edit:?} must be rejected"
            );
        }
    }

    #[test]
    fn learns_spell_with_spellbook_sets_available_flag() {
        let mut record = craft_record();
        // Spellbook slot word non-blank => carries a spellbook.
        let at = EQUIPMENT + 17 * EQUIPMENT_SLOT_LEN;
        record[at..at + 4].fill(0);
        let document = open_with_record(&record, 30_500);
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetSpellLearned {
                    spell_id: 15,
                    learned: true,
                }],
            )
            .expect("plans");
        assert_eq!(plan.len(), 2, "learned + available flag");
        assert_eq!(plan[0].offset, 30_500 + SPELLS_LEARNED + 15);
        assert_eq!(plan[1].offset, 30_500 + SPELLS_AVAILABLE + 15);
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].spells_learned[15], 1);
        assert_eq!(reopened.heroes[0].spells_available[15], 1);
    }

    #[test]
    fn unlearn_without_spellbook_leaves_available_untouched() {
        let mut record = craft_record();
        record[SPELLS_LEARNED + 9] = 1;
        record[SPELLS_AVAILABLE + 9] = 1;
        let document = open_with_record(&record, 30_500);
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetSpellLearned {
                    spell_id: 9,
                    learned: false,
                }],
            )
            .expect("plans");
        assert_eq!(plan.len(), 1, "only the learned flag is cleared");
        assert_eq!(plan[0].offset, 30_500 + SPELLS_LEARNED + 9);
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].spells_learned[9], 0);
        assert_eq!(reopened.heroes[0].spells_available[9], 1, "preserved");
    }

    #[test]
    fn refuses_edits_on_unsupported_layout_and_low_confidence() {
        // SoD-style version: recognized but no HotA hero layout.
        let mut payload = embed(&craft_record(), 30_500);
        payload[8..12].copy_from_slice(&42u32.to_le_bytes());
        payload[12..16].copy_from_slice(&1u32.to_le_bytes());
        let sod = SaveDocument::open(crate::patch::compress_gzip(&payload)).unwrap();
        assert_eq!(sod.format.compatibility, Compatibility::Readable);
        assert!(matches!(
            sod.plan_hero_edits(0, &[HeroEdit::SetAttack(9)]),
            Err(EditError::UnsupportedFormat { .. })
        ));

        // Low-confidence record: break the owner byte after location.
        let mut bad = craft_record();
        bad[OWNER] = 9;
        let document = open_with_record(&bad, 30_500);
        assert_eq!(document.heroes[0].confidence, crate::HeroConfidence::Low);
        assert!(matches!(
            document.plan_hero_edits(0, &[HeroEdit::SetAttack(9)]),
            Err(EditError::RecordNotEditable { id: 0 })
        ));
    }

    #[test]
    fn modified_roundtrip_changes_only_patched_bytes() {
        let record = craft_record();
        let document = open_with_record(&record, 30_500);
        let original_payload = document.payload().to_vec();
        let plan = document
            .plan_hero_edits(
                0,
                &[
                    HeroEdit::SetAttack(21),
                    HeroEdit::SetSpellLearned {
                        spell_id: 15,
                        learned: true,
                    },
                ],
            )
            .unwrap();
        let bytes = document.serialize_with_patches(&plan).unwrap();
        let reopened = SaveDocument::open(bytes).unwrap();
        assert_eq!(reopened.payload().len(), original_payload.len());
        let mut changed = Vec::new();
        for (index, (before, after)) in original_payload.iter().zip(reopened.payload()).enumerate()
        {
            if before != after {
                changed.push(index);
            }
        }
        let expected: Vec<usize> = plan.iter().map(|patch| patch.offset).collect();
        assert_eq!(changed, expected, "only patched offsets differ");
    }

    // ---------------------------------------------------------- skills --

    use crate::definitions::hero::{SKILL_COUNT_MAX, SKILL_SLOTS_LEGACY};

    fn record_with_skills(entries: &[(u8, u8)]) -> Vec<u8> {
        let mut record = craft_record();
        record[SKILL_COUNT] = entries.len() as u8;
        record[SKILL_COUNT + 1..SKILL_COUNT + 4].fill(0);
        record[SKILL_LEVELS..SKILL_LEVELS + 30].fill(0);
        record[SKILL_SLOTS_HOT_A..SKILL_SLOTS_HOT_A + 30].fill(0);
        for (index, &(id, mastery)) in entries.iter().enumerate() {
            record[SKILL_LEVELS + id as usize] = mastery;
            record[SKILL_SLOTS_HOT_A + id as usize] = (index + 1) as u8;
        }
        record
    }

    const AT: usize = 30_500;

    fn doc_with_skills(entries: &[(u8, u8)]) -> SaveDocument {
        let record = record_with_skills(entries);
        let payload = embed(&record, AT);
        SaveDocument::open(crate::patch::compress_gzip(&payload)).expect("opens")
    }

    fn relative_offsets(plan: &[BytePatch]) -> Vec<usize> {
        let mut offsets: Vec<usize> = plan.iter().map(|patch| patch.offset - AT).collect();
        offsets.sort_unstable();
        offsets
    }

    #[test]
    fn skills_add_plans_count_mastery_and_order_bytes() {
        let document = doc_with_skills(&[(7, 2), (24, 2), (8, 1)]);
        let edits = [HeroEdit::SetSkills {
            skills: vec![(7, 2), (24, 2), (8, 1), (2, 1)],
        }];
        let plan = document.plan_hero_edits(0, &edits).expect("plans");
        assert_eq!(
            relative_offsets(&plan),
            vec![SKILL_COUNT, SKILL_LEVELS + 2, SKILL_SLOTS_HOT_A + 2]
        );
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        let skills: Vec<(u8, u8, u8)> = reopened.heroes[0]
            .skills
            .iter()
            .map(|skill| (skill.skill_id, skill.mastery, skill.slot))
            .collect();
        assert_eq!(
            skills,
            vec![(7, 2, 1), (24, 2, 2), (8, 1, 3), (2, 1, 4)],
            "add keeps order and appends"
        );
    }

    #[test]
    fn skills_remove_clears_entry_and_renumbers() {
        let document = doc_with_skills(&[(7, 2), (24, 2), (8, 1)]);
        let edits = [HeroEdit::SetSkills {
            skills: vec![(24, 2), (8, 1)],
        }];
        let plan = document.plan_hero_edits(0, &edits).expect("plans");
        assert_eq!(
            relative_offsets(&plan),
            vec![
                SKILL_COUNT,
                SKILL_LEVELS + 7,
                SKILL_SLOTS_HOT_A + 7,
                SKILL_SLOTS_HOT_A + 8,
                SKILL_SLOTS_HOT_A + 24,
            ],
        );
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        let skills: Vec<(u8, u8, u8)> = reopened.heroes[0]
            .skills
            .iter()
            .map(|skill| (skill.skill_id, skill.mastery, skill.slot))
            .collect();
        assert_eq!(skills, vec![(24, 2, 1), (8, 1, 2)], "dense renumbering");
    }

    #[test]
    fn skills_mastery_change_touches_one_byte() {
        let document = doc_with_skills(&[(7, 2), (24, 2), (8, 1)]);
        let edits = [HeroEdit::SetSkills {
            skills: vec![(7, 3), (24, 2), (8, 1)],
        }];
        let plan = document.plan_hero_edits(0, &edits).expect("plans");
        assert_eq!(relative_offsets(&plan), vec![SKILL_LEVELS + 7]);
    }

    #[test]
    fn skills_reorder_touches_only_order_bytes() {
        let document = doc_with_skills(&[(7, 2), (24, 2), (8, 1)]);
        let edits = [HeroEdit::SetSkills {
            skills: vec![(8, 1), (7, 2), (24, 2)],
        }];
        let plan = document.plan_hero_edits(0, &edits).expect("plans");
        assert_eq!(
            relative_offsets(&plan),
            vec![
                SKILL_SLOTS_HOT_A + 7,
                SKILL_SLOTS_HOT_A + 8,
                SKILL_SLOTS_HOT_A + 24,
            ],
            "count and mastery bytes stay untouched"
        );
    }

    #[test]
    fn skills_reject_duplicates_invalid_values_and_capacity() {
        let document = doc_with_skills(&[(7, 2), (24, 2), (8, 1)]);
        let rejects = |skills: Vec<(u8, u8)>| {
            document
                .plan_hero_edits(0, &[HeroEdit::SetSkills { skills }])
                .unwrap_err()
        };
        rejects(vec![(7, 1), (7, 3)]);
        rejects(vec![(7, 0), (24, 2)]);
        rejects(vec![(7, 4), (24, 2)]);
        rejects(vec![(30, 1)]);
        let too_many: Vec<(u8, u8)> = (0..SKILL_COUNT_MAX as u8 + 1).map(|id| (id, 1)).collect();
        let error = rejects(too_many);
        assert!(
            error.to_string().contains("28"),
            "capacity explains: {error}"
        );
    }

    #[test]
    fn skills_preserve_legacy_region_and_unrecognized_entries() {
        let mut record = record_with_skills(&[(7, 2)]);
        // Unrecognized entries: mastery set but slot zero, and a slot beyond
        // the base count with zero mastery. The 26 legacy bytes at 212..238
        // keep craft_record's 0x55 filler.
        record[SKILL_LEVELS + 10] = 2;
        record[SKILL_SLOTS_HOT_A + 10] = 0;
        record[SKILL_LEVELS + 11] = 0;
        record[SKILL_SLOTS_HOT_A + 11] = 9;
        let payload = embed(&record, AT);
        let document = SaveDocument::open(crate::patch::compress_gzip(&payload)).expect("opens");
        let edits = [HeroEdit::SetSkills {
            skills: vec![(7, 2), (2, 1)],
        }];
        let plan = document.plan_hero_edits(0, &edits).expect("plans");
        let offsets = relative_offsets(&plan);
        assert!(
            offsets
                .iter()
                .all(|&offset| !(SKILL_SLOTS_LEGACY..SKILL_SLOTS_LEGACY + 26).contains(&offset)),
            "legacy region untouched: {offsets:?}"
        );
        for byte in [
            SKILL_LEVELS + 10,
            SKILL_SLOTS_HOT_A + 10,
            SKILL_LEVELS + 11,
            SKILL_SLOTS_HOT_A + 11,
        ] {
            assert!(
                !offsets.contains(&byte),
                "unrecognized entry {byte} preserved"
            );
        }
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        let reopened_record = &reopened.payload()[AT..AT + RECORD_LEN];
        assert_eq!(reopened_record[SKILL_LEVELS + 10], 2);
        assert_eq!(reopened_record[SKILL_SLOTS_HOT_A + 11], 9);
        assert!(
            reopened_record[SKILL_SLOTS_LEGACY..SKILL_SLOTS_LEGACY + 26]
                .iter()
                .all(|&byte| byte == 0x55),
            "legacy bytes byte-identical"
        );
    }

    // ------------------------------------------------------ equipment --

    use crate::EquipmentEdit;

    /// craft_record with game-native blank equipment (FF x8) and zeroed
    /// reservation bytes, so unequip is a true inverse of equip.
    fn record_for_equipment() -> Vec<u8> {
        let mut record = craft_record();
        for slot in 0..EQUIPMENT_SLOTS {
            let at = EQUIPMENT + slot * EQUIPMENT_SLOT_LEN;
            record[at..at + 8].fill(0xFF);
        }
        record
    }

    const SLOT_WEAPON: u8 = 3;
    const SLOT_SIDE1: u8 = 9;

    #[test]
    fn equipment_equip_updates_slot_and_attribute_bonus() {
        let document = open_with_record(&record_for_equipment(), AT);
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(7), // Centaur's Axe +2 Attack
                }],
            )
            .unwrap();
        assert_eq!(
            relative_offsets(&plan),
            vec![
                ATTACK,
                EQUIPMENT + SLOT_WEAPON as usize * EQUIPMENT_SLOT_LEN
            ],
            "slot word + attack bonus only"
        );
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].primary.attack, 5);
        assert_eq!(
            reopened.heroes[0].equipment_items[SLOT_WEAPON as usize],
            crate::EquipmentItem::Artifact(7)
        );
        // Unequip restores every byte (game-native blank equals the crafted one).
        let clear = reopened
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Empty,
                }],
            )
            .unwrap();
        assert_eq!(relative_offsets(&clear), vec![ATTACK, EQUIPMENT + 24]);
        let restored =
            SaveDocument::open(reopened.serialize_with_patches(&clear).unwrap()).unwrap();
        assert_eq!(restored.payload(), document.payload());
    }

    #[test]
    fn equipment_replace_recomputes_bonus_delta() {
        let mut record = record_for_equipment();
        let at = EQUIPMENT + SLOT_WEAPON as usize * EQUIPMENT_SLOT_LEN;
        record[at..at + 8].copy_from_slice(&[7, 0, 0, 0, 0, 0, 0, 0]);
        record[ATTACK] = 5; // base 3 + axe bonus 2
        let document = open_with_record(&record, AT);
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(11), // Sword of Hellfire +6 Attack
                }],
            )
            .unwrap();
        assert_eq!(relative_offsets(&plan), vec![ATTACK, EQUIPMENT + 24]);
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].primary.attack, 9);
        assert_eq!(
            reopened.heroes[0].equipment_items[SLOT_WEAPON as usize],
            crate::EquipmentItem::Artifact(11)
        );
    }

    #[test]
    fn equipment_grants_spells_without_touching_learned_flags() {
        let mut record = record_for_equipment();
        // Spellbook equipped (word zero) and Bless (41) learned.
        record[EQUIPMENT + 17 * EQUIPMENT_SLOT_LEN..EQUIPMENT + 17 * EQUIPMENT_SLOT_LEN + 4]
            .fill(0);
        record[SPELLS_LEARNED + 41] = 1;
        record[SPELLS_AVAILABLE + 41] = 1;
        let document = open_with_record(&record, AT);
        // Cure (37) is granted by the tome and not learned: its available
        // flag is the patched one. Bless stays available via the spellbook.
        let equip = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Artifact(89),
                }],
            )
            .unwrap();
        assert!(relative_offsets(&equip).contains(&(SPELLS_AVAILABLE + 37)));
        let reopened =
            SaveDocument::open(document.serialize_with_patches(&equip).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].spells_available[37], 1);
        assert_eq!(reopened.heroes[0].spells_available[41], 1);
        // Unequip: Bless remains available (spellbook) and learned.
        let clear = reopened
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Empty,
                }],
            )
            .unwrap();
        let restored =
            SaveDocument::open(reopened.serialize_with_patches(&clear).unwrap()).unwrap();
        assert_eq!(restored.heroes[0].spells_available[41], 1);
        assert_eq!(restored.heroes[0].spells_learned[41], 1);
        assert_eq!(restored.payload(), document.payload());
    }

    #[test]
    fn equipment_grant_removed_when_only_source_unequipped() {
        let document = open_with_record(&record_for_equipment(), AT);
        let equip = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Artifact(89), // Tome of Water
                }],
            )
            .unwrap();
        assert!(
            relative_offsets(&equip).contains(&(SPELLS_AVAILABLE + 41)),
            "Bless granted by the tome"
        );
        let reopened =
            SaveDocument::open(document.serialize_with_patches(&equip).unwrap()).unwrap();
        assert_eq!(reopened.heroes[0].spells_available[41], 1);
        let clear = reopened
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Empty,
                }],
            )
            .unwrap();
        let restored =
            SaveDocument::open(reopened.serialize_with_patches(&clear).unwrap()).unwrap();
        assert_eq!(
            restored.heroes[0].spells_available[41], 0,
            "grant withdrawn"
        );
        assert_eq!(restored.heroes[0].spells_learned[41], 0, "never set");
    }

    #[test]
    fn equipment_bannable_grants_respect_zero_available_flag() {
        let document = open_with_record(&record_for_equipment(), AT);
        // Summon Boat (0), Scuttle Boat (1) and Water Walk (7) are the
        // bannable spells; the save keeps them unavailable.
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Artifact(89),
                }],
            )
            .unwrap();
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        for spell in [0usize, 1, 7] {
            assert_eq!(
                reopened.heroes[0].spells_available[spell], 0,
                "bannable spell {spell} not granted"
            );
        }
        // A non-bannable grant was still written.
        assert_eq!(reopened.heroes[0].spells_available[41], 1);
    }

    #[test]
    fn equipment_rejects_bad_slots_artifacts_and_blocked_heroes() {
        let document = open_with_record(&record_for_equipment(), AT);
        let rejects = |edit: HeroEdit| document.plan_hero_edits(0, std::slice::from_ref(&edit));
        // Wrong slot kind: Lion's Shield of Courage is a shield, not a weapon.
        assert!(
            rejects(HeroEdit::SetEquipmentSlot {
                slot: SLOT_WEAPON,
                item: EquipmentEdit::Artifact(34),
            })
            .is_err()
        );
        // The Grail is inventory-only; spellbook, scroll, war machines and
        // IDs outside the catalog are never written as equipment.
        for id in [2, 0, 1, 4, 144, 166] {
            assert!(
                rejects(HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(id),
                })
                .is_err(),
                "artifact {id} rejected"
            );
        }
        // War machine and spellbook slots stay read-only.
        for slot in [13, 14, 15, 16, 17] {
            assert!(
                rejects(HeroEdit::SetEquipmentSlot {
                    slot,
                    item: EquipmentEdit::Artifact(7),
                })
                .is_err(),
                "slot {slot} rejected"
            );
        }
        // Unknown occupant: independence of other slots is not proven
        // (single-byte ID keeps the locator grammar valid:
        // 0xA6 is not a verified artifact).
        let mut unknown = record_for_equipment();
        unknown[EQUIPMENT..EQUIPMENT + 4].copy_from_slice(&166u32.to_le_bytes());
        let blocked = open_with_record(&unknown, AT);
        let reason = blocked.heroes[0].equipment_block_reason().unwrap();
        assert!(reason.contains("Helm (slot 1): Unknown artifact (ID 166)"));
        assert!(matches!(
            blocked.plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: 0, // the slot holding the unknown artifact
                    item: EquipmentEdit::Empty,
                }]
            ),
            Err(EditError::InvalidValue { .. })
        ));
        assert!(
            blocked
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetEquipmentSlot {
                        slot: SLOT_WEAPON,
                        item: EquipmentEdit::Artifact(7),
                    }]
                )
                .is_err(),
            "unknown effects cannot be proven independent"
        );
        // Reservation bytes no equipped combination explains block the section.
        let mut reserved = record_for_equipment();
        reserved[RESERVED + 4] = 1;
        let blocked = open_with_record(&reserved, AT);
        assert!(blocked.heroes[0].equipment_gating.reservation_mismatch);
        assert!(
            blocked.heroes[0]
                .equipment_block_reason()
                .unwrap()
                .contains("do not match")
        );
        // Backpack edits remain independent, including opaque reservations.
        let plan = blocked
            .plan_hero_edits(
                0,
                &[HeroEdit::SetBackpackSlot {
                    slot: 0,
                    item: BackpackEdit::Artifact(7),
                }],
            )
            .unwrap();
        let reopened = SaveDocument::open(blocked.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(
            reopened.heroes[0].equipment_gating.reservation_bytes,
            blocked.heroes[0].equipment_gating.reservation_bytes
        );
        assert_eq!(reopened.heroes[0].equipment, blocked.heroes[0].equipment);
        assert!(
            blocked
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetEquipmentSlot {
                        slot: SLOT_WEAPON,
                        item: EquipmentEdit::Artifact(7),
                    }]
                )
                .is_err()
        );
    }

    #[test]
    fn equipment_unmodeled_items_block_unproven_independent_changes() {
        // Hero wears a Tome of Water in a side slot (grants Cure 37) and an
        // unknown artifact in the helm.
        let mut record = record_for_equipment();
        let side = EQUIPMENT + SLOT_SIDE1 as usize * EQUIPMENT_SLOT_LEN;
        record[side..side + 8].copy_from_slice(&[89, 0, 0, 0, 0, 0, 0, 0]);
        record[EQUIPMENT..EQUIPMENT + 4].copy_from_slice(&166u32.to_le_bytes());
        let document = open_with_record(&record, AT);
        // Replacing the tome with a non-granting artifact would withdraw its
        // grants while an unmodeled item is worn: rejected.
        assert!(matches!(
            document.plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Artifact(125), // Shackles of War: no grants
                }]
            ),
            Err(EditError::InvalidValue { .. })
        ));
        // Grant additions are also blocked beside unmodeled equipment.
        let mut empty_side = record_for_equipment();
        empty_side[EQUIPMENT..EQUIPMENT + 4].copy_from_slice(&166u32.to_le_bytes());
        let empty_doc = open_with_record(&empty_side, AT);
        assert!(
            empty_doc
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetEquipmentSlot {
                        slot: SLOT_SIDE1,
                        item: EquipmentEdit::Artifact(87), // Tome of Air
                    }]
                )
                .is_err(),
            "unmodeled equipment blocks grant additions"
        );
        // Without the unmodeled helm, withdrawing grants is exact again (the
        // stored flag carries the tome's grant, as the game would save it).
        let mut clean = record_for_equipment();
        clean[side..side + 8].copy_from_slice(&[89, 0, 0, 0, 0, 0, 0, 0]);
        clean[SPELLS_AVAILABLE + 37] = 1;
        let clean_doc = open_with_record(&clean, AT);
        let plan = clean_doc
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_SIDE1,
                    item: EquipmentEdit::Artifact(125),
                }],
            )
            .unwrap();
        assert!(relative_offsets(&plan).contains(&(SPELLS_AVAILABLE + 37)));
    }

    #[test]
    fn equipment_no_op_and_conflicting_batches() {
        // No-op: already-empty slot stays untouched.
        let document = open_with_record(&record_for_equipment(), AT);
        assert!(
            document
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetEquipmentSlot {
                        slot: SLOT_WEAPON,
                        item: EquipmentEdit::Empty,
                    }]
                )
                .unwrap()
                .is_empty()
        );
        // Identical change computed by two edits collapses to one patch.
        let plan = document
            .plan_hero_edits(
                0,
                &[
                    HeroEdit::SetAttack(5),
                    HeroEdit::SetEquipmentSlot {
                        slot: SLOT_WEAPON,
                        item: EquipmentEdit::Artifact(7),
                    },
                ],
            )
            .unwrap();
        assert_eq!(relative_offsets(&plan), vec![ATTACK, EQUIPMENT + 24]);
        // Divergent values for the same byte are an explicit error.
        let conflict = document.plan_hero_edits(
            0,
            &[
                HeroEdit::SetAttack(4),
                HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(7),
                },
            ],
        );
        assert!(
            matches!(conflict, Err(EditError::InvalidValue { .. })),
            "{conflict:?}"
        );
        // Two different artifacts for one slot in one batch conflict too.
        let conflict = document.plan_hero_edits(
            0,
            &[
                HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(7),
                },
                HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(11),
                },
            ],
        );
        assert!(matches!(conflict, Err(EditError::InvalidValue { .. })));
    }

    /// Record wearing Cloak of the Undead King (130) with its neck and feet
    /// counters set, as the game stores it in the 44/11 samples.
    fn record_with_undead_king_cloak() -> Vec<u8> {
        let mut record = record_for_equipment();
        let cloak = EQUIPMENT + EQUIPMENT_SLOT_LEN;
        record[cloak..cloak + 8].copy_from_slice(&[130, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        record[RESERVED + 3] = 1; // neck
        record[RESERVED + 8] = 1; // feet
        record
    }

    #[test]
    fn combination_equip_writes_reservation_counters_and_bonus() {
        let document = open_with_record(&record_for_equipment(), AT);
        let base = document.heroes[0].primary;
        // Power of the Dragon Father in the armor slot.
        let equip = [HeroEdit::SetEquipmentSlot {
            slot: 5,
            item: EquipmentEdit::Artifact(134),
        }];
        let plan = document.plan_hero_edits(0, &equip).unwrap();
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        let hero = &reopened.heroes[0];
        // Same counters the game wrote for this artifact in the samples.
        assert_eq!(
            hero.equipment_gating.reservation_bytes,
            [0, 1, 1, 1, 1, 1, 0, 2, 1, 0]
        );
        assert!(hero.equipment_gating.is_clear());
        assert_eq!(hero.primary.attack, base.attack.wrapping_add(16));
        assert_eq!(hero.primary.knowledge, base.knowledge.wrapping_add(16));
        // Reserved kinds cannot be filled while the combination is worn.
        let blocked = reopened.plan_hero_edits(
            0,
            &[HeroEdit::SetEquipmentSlot {
                slot: SLOT_WEAPON,
                item: EquipmentEdit::Artifact(7),
            }],
        );
        assert!(
            matches!(&blocked, Err(EditError::InvalidValue { reason, .. })
                if reason.contains("Power of the Dragon Father")),
            "{blocked:?}"
        );
        // Unequip is the exact inverse.
        let plan = reopened
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: 5,
                    item: EquipmentEdit::Empty,
                }],
            )
            .unwrap();
        let restored = SaveDocument::open(reopened.serialize_with_patches(&plan).unwrap()).unwrap();
        assert_eq!(restored.payload(), document.payload());
    }

    #[test]
    fn combination_needs_free_slots_and_swaps_in_one_batch() {
        let mut record = record_for_equipment();
        let weapon = EQUIPMENT + SLOT_WEAPON as usize * EQUIPMENT_SLOT_LEN;
        record[weapon..weapon + 8].copy_from_slice(&[7, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        let document = open_with_record(&record, AT);
        // Armor of the Damned reserves helm, weapon and shield.
        let equip = HeroEdit::SetEquipmentSlot {
            slot: 5,
            item: EquipmentEdit::Artifact(132),
        };
        let rejected = document.plan_hero_edits(0, std::slice::from_ref(&equip));
        assert!(
            matches!(&rejected, Err(EditError::InvalidValue { reason, .. })
                if reason.contains("Centaur's Axe")),
            "{rejected:?}"
        );
        // Freeing the weapon slot in the same batch makes it valid; the
        // attack byte moves by the joint delta (-2 axe, +3 armor).
        let plan = document
            .plan_hero_edits(
                0,
                &[
                    HeroEdit::SetEquipmentSlot {
                        slot: SLOT_WEAPON,
                        item: EquipmentEdit::Empty,
                    },
                    equip,
                ],
            )
            .unwrap();
        let reopened = SaveDocument::open(document.serialize_with_patches(&plan).unwrap()).unwrap();
        let hero = &reopened.heroes[0];
        assert_eq!(
            hero.equipment_gating.reservation_bytes,
            [0, 1, 0, 0, 1, 1, 0, 0, 0, 0]
        );
        assert_eq!(
            hero.primary.attack,
            document.heroes[0].primary.attack.wrapping_add(1)
        );
    }

    #[test]
    fn worn_combination_stays_editable_and_side_capacity_is_counted() {
        let document = open_with_record(&record_with_undead_king_cloak(), AT);
        assert!(document.heroes[0].equipment_gating.is_clear());
        // An unrelated slot is editable beside a worn combination and the
        // reservation counters are left alone.
        let plan = document
            .plan_hero_edits(
                0,
                &[HeroEdit::SetEquipmentSlot {
                    slot: SLOT_WEAPON,
                    item: EquipmentEdit::Artifact(147), // Trident of Dominion
                }],
            )
            .unwrap();
        assert_eq!(relative_offsets(&plan), vec![ATTACK, EQUIPMENT + 24]);
        // The reserved neck slot rejects a necklace.
        assert!(
            document
                .plan_hero_edits(
                    0,
                    &[HeroEdit::SetEquipmentSlot {
                        slot: 2,
                        item: EquipmentEdit::Artifact(33),
                    }]
                )
                .is_err()
        );
        // Bow of the Sharpshooter (side + 2 side) fits with two other side
        // items, not with three.
        let side = |slot: u8, id: u32| HeroEdit::SetEquipmentSlot {
            slot,
            item: EquipmentEdit::Artifact(id),
        };
        assert!(
            document
                .plan_hero_edits(0, &[side(9, 137), side(10, 153), side(11, 154)])
                .is_ok()
        );
        assert!(
            document
                .plan_hero_edits(
                    0,
                    &[side(9, 137), side(10, 153), side(11, 154), side(12, 155)]
                )
                .is_err()
        );
    }
}

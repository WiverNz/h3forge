import type { ReasonDescriptor } from '../i18n/reasons';
/** Keep aligned with src-tauri/src/dto.rs. */
export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  architecture: string;
}

export interface AppErrorShape {
  code: string;
  message: string;
  params?: Record<string, number>;
  cause?: string;
}

/** Raw bytes plus the core lossy decode; re-decode `raw` with the selected legacy encoding. */
export interface TextDto {
  raw: number[];
  lossy: string;
}

export interface SaveGrantDto {
  grantId: string;
  fileName: string;
}

export interface DiagnosticDto {
  code: string;
  message: string;
}

export interface SaveSummaryDto {
  saveDisabledReason: ReasonDescriptor | null;
  fileName: string;
  familyTitle: string | null;
  major: number;
  minor: number;
  containerKind: 'regular' | 'campaign' | 'chronicles' | string;
  compatibility: 'verified' | 'readable' | 'unsupported' | string;
  heroesSupported: boolean;
  mapName: TextDto | null;
  mapDescription: TextDto | null;
  heroCount: number;
  confirmedCount: number;
  diagnostics: DiagnosticDto[];
}

export interface OwnerDto {
  kind: 'player' | 'neutral';
  player: number | null;
  label: string;
}

export interface HeroListItemDto {
  armyCreatureIds: number[];
  id: number;
  name: TextDto;
  owner: OwnerDto;
  confidence: 'high' | 'low';
  level: number;
  hasSpellbook: boolean;
  hasNotes: boolean;
  duplicateName: boolean;
}

export interface PrimaryDto {
  attack: number;
  defense: number;
  spellPower: number;
  knowledge: number;
}

export interface HeroSkillDto {
  skillId: number;
  name: string | null;
  mastery: number;
  masteryName: string | null;
  slot: number;
}

export interface SpellDto {
  id: number;
  name: string | null;
}

export interface ArmySlotDto {
  creatureId: number;
  count: number;
  name: string | null;
  empty: boolean;
  editable: boolean;
}

export interface CreatureDto {
  id: number;
  name: string;
}

export interface EquipmentDto {
  slot: string;
  bytes: number[];
}

export interface HeroNoteDto {
  kind: string;
  message: string;
}

export interface HeroDetailDto {
  /** Current save coordinates; null for an unverified or off-map position. */
  coordinates: [number, number, number] | null;
  id: number;
  spanStart: number;
  spanEnd: number;
  name: TextDto;
  owner: OwnerDto;
  confidence: 'high' | 'low';
  level: number;
  levelFromExperience: number;
  experience: number;
  spellPoints: number;
  movementTotal: number;
  movementLeft: number;
  primary: PrimaryDto;
  skills: HeroSkillDto[];
  spellsLearned: SpellDto[];
  spellsAvailable: SpellDto[];
  hasSpellbook: boolean;
  army: ArmySlotDto[];
  equipment: EquipmentDto[];
  /** Flat 64 x 8 raw backpack bytes. */
  inventory: number[];
  backpack: BackpackItemDto[];
  notes: HeroNoteDto[];
  /** Initial-map reference facts; null without a linked map. */
  map: HeroMapInfoDto | null;
}

export type ArchiveStateDto =
  | { kind: 'readable'; entries: number }
  | { kind: 'unsupported'; reason: string }
  | { kind: 'unreadable'; reason: string };

export interface ArchiveStatusDto {
  name: string;
  state: ArchiveStateDto;
}

export interface InstallationDto {
  pickedRoot: string;
  archiveRoot: string;
  source: 'exe-parent' | 'development' | 'stored' | 'manual' | string;
  resourceSets: string[];
  hasBaseArtwork: boolean;
  archives: ArchiveStatusDto[];
}

export interface PortraitIdentityDto {
  heroType: number;
  heroClass: number;
  portraitId: number;
}

export interface PortraitDto {
  resolved: boolean;
  identity: PortraitIdentityDto | null;
  pngBase64?: string;
  reason: string | null;
}

/** Keep aligned with dto.rs HeroEditRequestDto. All fields optional; present fields are absolute desired values. */
export interface BackpackItemDto {
  slot: number;
  kind: 'empty' | 'artifact' | 'scroll' | 'unknown';
  artifactId: number | null;
  name: string | null;
  spellId: number | null;
  spellName: string | null;
  editable: boolean;
}
export type BackpackEdit = { kind: 'empty' } | { kind: 'artifact' | 'scroll'; value: number };

export interface EquipmentItemDto {
  kind: 'empty' | 'artifact' | 'scroll' | 'unknown';
  artifactId: number | null;
  name: string | null;
  spellId: number | null;
  spellName: string | null;
}

export interface EquipmentSlotDto {
  slot: number;
  name: string;
  /** "helm" | "cloak" | "neck" | "weapon" | "shield" | "armor" | "hand" | "feet" | "side" | "machine" | "spellbook". */
  kind: string;
  item: EquipmentItemDto;
  /** Name of the worn combination artifact that takes this empty slot. */
  reservedBy: string | null;
  editable: boolean;
  readOnlyReason: ReasonDescriptor | null;
}

export interface HeroEditRequest {
  coordinates?: [number, number, number];
  backpackSlot?: { slot: number; item: BackpackEdit };
  backpackDestination?: { slot: number; item: BackpackEdit };
  equipmentSlot?: { slot: number; item: { kind: 'empty' } | { kind: 'artifact'; value: number } };
  /** Second equipment slot changed in the same batch. */
  equipmentSecond?: { slot: number; item: { kind: 'empty' } | { kind: 'artifact'; value: number } };
  attack?: number;
  defense?: number;
  spellPower?: number;
  knowledge?: number;
  experience?: number;
  level?: number;
  spellPoints?: number;
  spell?: { id: number; learned: boolean };
  army?: { slot: number; creatureId: number | null; count: number };
  /** Absolute ordered secondary-skill list replacing the current set. */
  skills?: { id: number; mastery: number }[];
}

export interface HeroEditResponseDto {
  coordinates: [number, number, number] | null;
  coordinatesReadOnlyReason: ReasonDescriptor | null;
  id: number;
  editable: boolean;
  readOnlyReason: ReasonDescriptor | null;
  primary: PrimaryDto;
  experience: number;
  level: number;
  spellPoints: number;
  /** Effective learned spells (base flags plus pending overrides). */
  spellsLearned: SpellDto[];
  /** All toggleable spell choices. */
  spellChoices: SpellDto[];
  /** Effective secondary skills in display order (pending override applied). */
  skills: HeroSkillDto[];
  /** All selectable secondary skills by binary ID. */
  skillChoices: { id: number; name: string | null }[];
  /** Why skill editing is unavailable, when it is. */
  skillsReadOnlyReason: ReasonDescriptor | null;
  backpack: BackpackItemDto[];
  artifactChoices: { id: number; name: string; equipKind: string | null; effect: string | null; combination: boolean; alsoTakes: string[]; partOf: string | null }[];
  backpackReadOnlyReason: ReasonDescriptor | null;
  /** Effective decoded equipment (base slots plus pending overrides). */
  equipment: EquipmentSlotDto[];
  /** Why equipment editing is unavailable, when it is. */
  equipmentReadOnlyReason: ReasonDescriptor | null;
  /** Equipment gating diagnostics. */
  equipmentLimitations: string[];
  army: ArmySlotDto[];
  creatureChoices: CreatureDto[];
  armyReadOnlyReason: ReasonDescriptor | null;
  /** Whether this hero is marked for dismissal; applied by Save. */
  dismissalPending: boolean;
  /** Why the hero cannot be dismissed; null when available or already pending. */
  dismissalBlockedReason: ReasonDescriptor | null;
  /** Labels of the fields with pending edits, e.g. "Attack" or "Spell 15 (Fireball)". */
  dirtyFields: string[];
  warnings: string[];
}

export interface PendingEditDto {
  recordKind: 'hero' | 'town' | string;
  recordId: number;
  /** Raw record name; decode with decodeText. */
  recordName: TextDto;
  field: string;
  before: string;
  after: string;
}

// ----------------------------------------------------------- H3F-013 towns --

export interface HeroRefDto {
  id: number;
  /** Raw name bytes of the referenced hero record; decode with decodeText. */
  name: TextDto;
}

export interface TownListItemDto {
  id: number;
  name: TextDto;
  owner: OwnerDto;
  confidence: 'high' | 'low';
  townType: number;
  townTypeName: string | null;
  garrisonCreatureIds: number[];
  hasGarrisonHero: boolean;
  hasVisitingHero: boolean;
  hasNotes: boolean;
  duplicateName: boolean;
  /** Fort built: selects the fort or village town icon. */
  hasFort: boolean;
}

export interface TownNoteDto {
  kind: string;
  message: string;
}

/** What the source map says about the town at this position (scenario start). */
export interface TownMapInfoDto {
  matched: boolean;
  ambiguous: boolean;
  mapName: TextDto | null;
  nameMatches: boolean;
  mapOwner: number | null;
  ownerMatches: boolean | null;
  mapFaction: number | null;
  factionMatches: boolean | null;
  mapHasGarrison: boolean;
}

export interface TownDetailDto {
  id: number;
  spanStart: number;
  spanEnd: number;
  name: TextDto;
  owner: OwnerDto;
  confidence: 'high' | 'low';
  townType: number;
  townTypeName: string | null;
  x: number;
  y: number;
  z: number;
  mageGuildLevel: number;
  hasFort: boolean;
  garrison: ArmySlotDto[];
  garrisonedHero: HeroRefDto | null;
  visitingHero: HeroRefDto | null;
  notes: TownNoteDto[];
  map: TownMapInfoDto | null;
}

/** Keep aligned with dto.rs TownEditRequestDto. */
export interface TownEditRequest {
  army?: { slot: number; creatureId: number | null; count: number };
  /** Construct or remove one building by building ID. */
  building?: { id: number; built: boolean };
  /** Absolute spell list of one Mage Guild level 1-5, in slot order. */
  guild?: { level: number; spells: number[] };
}

/** Effective army of a town's garrisoned hero; edited through the hero's own army edit. */
export interface GarrisonHeroArmyDto {
  heroId: number;
  slots: ArmySlotDto[];
  readOnlyReason: ReasonDescriptor | null;
  /** The hero's pending army labels, "Army slot N". */
  dirtyFields: string[];
}

/** One building with its effective state (base plus pending edits). */
export interface TownBuildingDto {
  id: number;
  name: string;
  group: 'halls' | 'fortifications' | 'mageGuild' | 'economy' | 'dwellings' | 'special' | 'other';
  built: boolean;
  /** The saved game allows constructing it in this town. */
  allowed: boolean;
  editable: boolean;
  lockReason: ReasonDescriptor | null;
  dirty: boolean;
}

export interface TownBuildingsDto {
  /** Town type, for building pictures. */
  townType: number;
  readOnlyReason: ReasonDescriptor | null;
  items: TownBuildingDto[];
}

export interface GuildLevelDto {
  level: number;
  built: boolean;
  /** The level can exist in this town. */
  available: boolean;
  capacity: number;
  spells: SpellDto[];
  editable: boolean;
  dirty: boolean;
}

export interface GuildSpellChoiceDto {
  id: number;
  name: string;
  level: number;
  /** Banned in this town by the saved game; cannot be added. */
  banned: boolean;
}

/** Current saved-game Mage Guild state, never the map's must/may masks. */
export interface TownGuildDto {
  readOnlyReason: ReasonDescriptor | null;
  builtLevel: number;
  maxLevel: number;
  hasLibrary: boolean;
  levels: GuildLevelDto[];
  spellChoices: GuildSpellChoiceDto[];
}

export interface TownEditResponseDto {
  id: number;
  editable: boolean;
  readOnlyReason: ReasonDescriptor | null;
  garrison: ArmySlotDto[];
  creatureChoices: CreatureDto[];
  garrisonReadOnlyReason: ReasonDescriptor | null;
  garrisonedHero: HeroRefDto | null;
  visitingHero: HeroRefDto | null;
  /** The garrisoned hero's own army, which is the garrison while that hero is inside. */
  garrisonHeroArmy: GarrisonHeroArmyDto | null;
  /** Null when the save layout has no verified building offsets. */
  buildings: TownBuildingsDto | null;
  guild: TownGuildDto | null;
  dirtyFields: string[];
  warnings: string[];
}

/** What the source map says about one hero type (scenario start, not current state). */
export interface HeroMapInfoDto {
  placement: 'hero' | 'prison' | 'not-placed' | 'ambiguous' | 'unknown';
  x: number | null;
  y: number | null;
  /** 0 surface, 1 underground. */
  level: number | null;
  owner: number | null;
  mapName: TextDto | null;
  mapPortraitId: number | null;
  portraitMatches: boolean | null;
}

export interface MapCandidateDto {
  index: number;
  fileName: string;
  folder: string;
}

export interface LinkedMapDto {
  fileName: string;
  folder: string;
  name: TextDto;
  format: string;
  hotaRelease: string | null;
  size: number;
  hasUnderground: boolean;
  matchStrength: 'full' | 'name-and-dimensions' | 'name-only';
  playablePlayers: number;
  humanPlayers: number;
  towns: number | null;
  placedHeroes: number | null;
  randomHeroes: number | null;
  objects: number | null;
  bodyError: string | null;
  heroesPlacedInSave: number;
  portraitsChecked: number;
  portraitsConsistent: number;
  /** Save towns matched to exactly one map town object by coordinates. */
  townsMatchedInSave: number;
}

export interface MapLinkDto {
  status: 'matched' | 'manual' | 'ambiguous' | 'not-found' | 'no-map-name';
  message: string;
  searched: string[];
  scannedFiles: number;
  candidates: MapCandidateDto[];
  map: LinkedMapDto | null;
}

export interface PlayerDto {
  status: 'active' | 'defeated' | 'unused' | 'unknown';
  statusReason: string;
  details: PlayerDetailsDto;
  id: number;
  color: number;
  name: string;
  active: boolean;
  heroCount: number;
  townCount: number;
  original: number[];
  resources: number[];
  readOnlyReason: ReasonDescriptor | null;
  dirtyResources: number[];
}

export interface PlayerDetailsDto {
  mapLinked: boolean;
  eligibility: string | null;
  controller: string | null;
  controllerReason: string;
  team: number | null;
  allies: number[] | null;
  enemies: number[] | null;
  teamSource: string;
  startingTown: { name: TextDto | null; faction: number | null; factionName: string; x: number; y: number; z: number } | null;
  startingHero: { heroType: number; name: TextDto | null; portraitId: number | null; portraitSource: string } | null;
  startingHeroReason: string;
  mapSize: number | null;
  levels: number | null;
  obelisks: { mapObjectId: number; x: number; y: number; z: number }[] | null;
  grailPlacements: number | null;
  puzzleProgress: number | null;
  visitedObelisks: number | null;
  revealMapReason: string;
  revealPuzzleReason: string;
}
export interface NeutralArmyDto {
  allowedCreatures: number[];
  id: number;
  kind: 'wandering' | 'garrison';
  position: [number, number, number];
  creatureId: number | null;
  name: string | null;
  army: ArmySlotDto[] | null;
  owner: number | null;
  readOnlyReason: string | null;
  dirtySlots: number[];
  initialArmy: { creatureId: number | null; name: string | null; count: number }[] | null;
}

export interface NeutralListDto {
  groups: NeutralArmyDto[];
  unavailableReason: string | null;
  choices: CreatureDto[];
  initialGroups: number | null;
}

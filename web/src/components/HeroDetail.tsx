import { ReasonText, reasonText } from '../i18n/reasons';
import { colorName } from '../i18n/labels';
import { t, i18n } from '../i18n/index';
import { playerColor } from '../lib/playerColor';
import { useEffect, useRef, useState } from 'react';
import type {
  HeroDetailDto,
  HeroEditRequest,
  HeroEditResponseDto,
  PortraitDto,
  SpellDto,
} from '../types/api';
import { decodeText, toHex, type LegacyEncoding } from '../lib/legacyText';
import { getHeroPortrait } from '../api/resources';
import { ArmyEditor } from './ArmyEditor';
import { CoordinatesEditor } from './CoordinatesEditor';
import { Portrait } from './Portrait';
import { BackpackEditor } from './BackpackEditor';
import { EquipmentEditor } from './EquipmentEditor';
import { SkillsEditor } from './SkillsEditor';
import { SpellIcon } from './SpellIcon';

const TABS = [
  { key: 'general', get label() { return t('ui.general'); } },
  { key: 'spells', get label() { return t('ui.spells'); } },
  { key: 'skills', get label() { return t('ui.skills'); } },
  { key: 'army', get label() { return t('ui.army'); } },
  { key: 'artifacts', get label() { return t('ui.artifacts'); } },
] as const;
type TabKey = (typeof TABS)[number]['key'];

/**
 * Hero inspection with validated editing, organized into compact tabs in the
 * spirit of the original map editor. All panels stay mounted while hidden,
 * so in-progress input and unsaved edits survive tab switches. Primary
 * attributes, experience, level, spell points, learned spells and armies are
 * editable for confirmed records on a supported layout; everything else is
 * honestly read-only. Values shown are effective values (base plus pending).
 */
export function HeroDetail({
  hero,
  encoding,
  edit,
  onApplyEdit,
  onDiscardHero,
  onSetDismissal,
  tabRequest,
}: {
  hero: HeroDetailDto;
  encoding: LegacyEncoding;
  edit: HeroEditResponseDto | null;
  onApplyEdit: (request: HeroEditRequest) => void;
  onDiscardHero: () => void;
  /** Marks (true) or unmarks (false) this hero for dismissal. */
  onSetDismissal: (dismissed: boolean) => void;
  /** Opens a tab for one hero, e.g. Army when arriving from a town; `nonce` repeats a request. */
  tabRequest?: { heroId: number; tab: TabKey; nonce: number } | null;
}) {
  const [portrait, setPortrait] = useState<PortraitDto | null>(null);
  const [tab, setTab] = useState<TabKey>('general');
  const [artifactTarget, setArtifactTarget] = useState<{ hero: number; area: 'equipment' | 'backpack'; slot: number } | null>(null);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const requestedHero = tabRequest?.heroId;
  const requestedTab = tabRequest?.tab;
  const requestNonce = tabRequest?.nonce;
  const handledRequest = useRef<number | null>(null);
  useEffect(() => {
    // Apply each request once, when its hero is shown; later manual tab
    // choices and hero switches are not overridden.
    if (!requestedTab || requestedHero !== hero.id || requestNonce === undefined) return;
    if (handledRequest.current === requestNonce) return;
    handledRequest.current = requestNonce;
    setTab(requestedTab);
  }, [requestedHero, requestedTab, requestNonce, hero.id]);

  useEffect(() => {
    let active = true;
    setPortrait(null);
    getHeroPortrait(hero.id, 'large').then(
      (result) => { if (active) setPortrait(result); },
      () => { if (active) setPortrait(null); },
    );
    return () => { active = false; };
  }, [hero.id]);

  const [confirmDismissal, setConfirmDismissal] = useState(false);
  useEffect(() => { setConfirmDismissal(false); }, [hero.id]);

  const editable = edit?.editable === true;
  const current = edit?.id === hero.id ? edit : null;
  const dismissalPending = current?.dismissalPending === true;
  const dismissalBlocked = current?.dismissalBlockedReason ?? null;
  const dirty = new Set(edit?.dirtyFields ?? []);
  const dirtySpells = new Set(
    (edit?.dirtyFields ?? [])
      .filter((field) => field.startsWith("Spell "))
      .map((field) => Number(field.split(' ')[1]))
      .filter((id) => Number.isFinite(id)),
  );
  const learnedIds = new Set((edit?.spellsLearned ?? []).map((spell) => spell.id));
  const values = {
    attack: edit?.primary.attack ?? hero.primary.attack,
    defense: edit?.primary.defense ?? hero.primary.defense,
    spellPower: edit?.primary.spellPower ?? hero.primary.spellPower,
    knowledge: edit?.primary.knowledge ?? hero.primary.knowledge,
    experience: edit?.experience ?? hero.experience,
    level: edit?.level ?? hero.level,
    spellPoints: edit?.spellPoints ?? hero.spellPoints,
  };

  const selectTabByOffset = (offset: number) => {
    const count = TABS.length;
    const next = (TABS.findIndex((entry) => entry.key === tab) + offset + count) % count;
    const entry = TABS[next];
    if (entry) {
      setTab(entry.key);
      tabRefs.current[next]?.focus();
    }
  };

  const onTabKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      selectTabByOffset(1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      selectTabByOffset(-1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      const first = TABS[0];
      if (first) {
        setTab(first.key);
        tabRefs.current[0]?.focus();
      }
    } else if (event.key === 'End') {
      event.preventDefault();
      const last = TABS[TABS.length - 1];
      if (last) {
        setTab(last.key);
        tabRefs.current[TABS.length - 1]?.focus();
      }
    }
  };

  return (
    <article className="hero-detail" aria-labelledby="hero-detail-title">
      <header className="hero-detail-head" data-player-color={playerColor(hero.owner)}>
        <div className="hero-detail-id">
          <Portrait portrait={portrait} size="large" />
          <div>
            <p className="eyebrow">{t('ui.levelValue', { value1: values.level })}</p>
            <h2 id="hero-detail-title">{decodeText(hero.name, encoding)}</h2>
            {hero.map && (
              <p className="hero-map-origin"
                title={t('ui.fromTheSourceMapH3mWhereThisHeroType')}>
                {describeMapOrigin(hero.map, encoding)}
              </p>
            )}
          </div>
        </div>
        <div className="hero-badges">
          <span className={`owner owner-${hero.owner.kind}`}>{colorName(hero.owner.player)}</span>
          {hero.confidence === 'low' && <span className="flag flag-warn">{t('ui.lowConfidence')}</span>}
          {editable ? (
            <span className="badge-editable">{t('ui.editable')}</span>
          ) : (
            <span className="badge-readonly" title={reasonText(edit?.readOnlyReason)}>
              {t('ui.readOnly')}</span>
          )}
          {editable && dirty.size > 0 && (
            <button type="button" className="revert-button" onClick={onDiscardHero}>{t('ui.revertChanges')}</button>
          )}
          {hero.owner.kind === 'player' && !dismissalPending && (
            <button type="button" className="dismiss-button"
              disabled={!editable || !current || dismissalBlocked !== null || confirmDismissal}
              title={dismissalBlocked ? `${t('ui.dismissHeroUnavailable')} ${dismissalBlocked.details}` : undefined}
              onClick={() => setConfirmDismissal(true)}>{t('ui.dismissHero')}</button>
          )}
        </div>
      </header>

      {confirmDismissal && !dismissalPending && !dismissalBlocked && (
        <div className="dismiss-panel" role="alertdialog" aria-labelledby="dismiss-title" aria-describedby="dismiss-text">
          <p id="dismiss-title" className="dismiss-title">
            {t('ui.dismissHeroQuestion', { name: decodeText(hero.name, encoding) })}</p>
          <p id="dismiss-text" className="fineprint">{t('ui.dismissHeroExplain')}</p>
          <p className="fineprint">{t('ui.dismissHeroDeviation')}</p>
          <div className="dismiss-actions">
            <button type="button" className="dismiss-confirm"
              onClick={() => { setConfirmDismissal(false); onSetDismissal(true); }}>{t('ui.dismissHeroConfirm')}</button>
            <button type="button" className="revert-button" autoFocus
              onClick={() => setConfirmDismissal(false)}>{t('ui.dismissHeroCancel')}</button>
          </div>
        </div>
      )}
      {dismissalPending && (
        <div className="dismiss-panel" role="status">
          <p className="dismiss-title">{t('ui.dismissHeroPending')}</p>
          <p className="fineprint">{t('ui.dismissHeroDeviation')}</p>
          <div className="dismiss-actions">
            <button type="button" className="revert-button" onClick={() => onSetDismissal(false)}>{t('ui.dismissHeroKeep')}</button>
          </div>
        </div>
      )}
      {editable && current && hero.owner.kind === 'player' && dismissalBlocked && (
        <p className="fineprint edit-blocked">{t('ui.dismissHeroUnavailable')} <span className="feedback-details">{dismissalBlocked.details}</span></p>
      )}

      {edit && !edit.editable && edit.readOnlyReason && (
        <p className="fineprint edit-blocked">{<ReasonText reason={edit.readOnlyReason} />}</p>
      )}
      {edit && edit.warnings.length > 0 && (
        <ul className="hero-notes edit-warnings" aria-label={t('ui.editWarnings')}>
          {edit.warnings.map((warning, index) => (
            <li key={index}>{warning}</li>
          ))}
        </ul>
      )}
      <div className="detail-tabs" role="tablist" aria-label={t('ui.heroSections')}>
        {TABS.map((entry, index) => (
          <button
            key={entry.key}
            type="button"
            role="tab"
            id={`tab-${entry.key}`}
            aria-controls={`panel-${entry.key}`}
            aria-selected={tab === entry.key}
            tabIndex={tab === entry.key ? 0 : -1}
            className={tab === entry.key ? 'detail-tab-active' : undefined}
            ref={(node) => {
              tabRefs.current[index] = node;
            }}
            onClick={() => setTab(entry.key)}
            onKeyDown={onTabKeyDown}
          >
            {entry.label}
          </button>
        ))}
      </div>

      <div
        role="tabpanel"
        id="panel-general"
        aria-labelledby="tab-general"
        hidden={tab !== 'general'}
        className="detail-tab"
      >
        <CoordinatesEditor key={hero.id} value={current?.coordinates ?? hero.coordinates}
          blocked={current?.coordinatesReadOnlyReason ?? (!editable || !current ? { code: 'read-only', details: t('ui.heroRelocationBlocked') } : null)}
          dirty={dirty.has('Coordinates')} onApply={coordinates => onApplyEdit({ coordinates })} />
        {editable ? (
          <dl className="facts facts-editable general-grid">
            <div className="general-group-title"><dt>{t('ui.progressionMagic')}</dt></div>
            <NumberField
              label={t('ui.level')}
              value={values.level}
              min={1}
              max={74}
              dirty={dirty.has("Level")}
              onCommit={(level) => onApplyEdit({ level })}
            />
            <NumberField
              label={t('ui.experience')}
              value={values.experience}
              min={0}
              max={4294967295}
              dirty={dirty.has("Experience")}
              onCommit={(experience) => onApplyEdit({ experience })}
            />
            <NumberField
              label={t('ui.spellPoints')}
              value={values.spellPoints}
              min={0}
              max={65535}
              dirty={dirty.has("Spell points")}
              onCommit={(spellPoints) => onApplyEdit({ spellPoints })}
            />
            <div>
              <dt>{t('ui.movementReadOnly')}</dt>
              <dd className="fact-value">
                {hero.movementLeft.toLocaleString(i18n.resolvedLanguage ?? 'en')} / {hero.movementTotal.toLocaleString(i18n.resolvedLanguage ?? 'en')}
              </dd>
            </div>
            <div className="general-group-title"><dt>{t('ui.primaryAttributes')}</dt></div>
            <NumberField
              label={t('ui.attack')}
              value={values.attack}
              min={0}
              max={99}
              dirty={dirty.has("Attack")}
              onCommit={(attack) => onApplyEdit({ attack })}
            />
            <NumberField
              label={t('ui.defense')}
              value={values.defense}
              min={0}
              max={99}
              dirty={dirty.has("Defense")}
              onCommit={(defense) => onApplyEdit({ defense })}
            />
            <NumberField
              label={t('ui.spellPower')}
              value={values.spellPower}
              min={1}
              max={99}
              dirty={dirty.has("Spell Power")}
              onCommit={(spellPower) => onApplyEdit({ spellPower })}
            />
            <NumberField
              label={t('ui.knowledge')}
              value={values.knowledge}
              min={1}
              max={99}
              dirty={dirty.has("Knowledge")}
              onCommit={(knowledge) => onApplyEdit({ knowledge })}
            />
          </dl>
        ) : (
          <dl className="facts general-grid">
            <div>
              <dt>{t('ui.level')}</dt>
              <dd className="fact-value">
                {hero.level}
                {hero.levelFromExperience !== hero.level &&
                  t('ui.experienceImpliesValue', { value1: hero.levelFromExperience })}
              </dd>
            </div>
            <div>
              <dt>{t('ui.experience')}</dt>
              <dd className="fact-value">{hero.experience.toLocaleString(i18n.resolvedLanguage ?? 'en')}</dd>
            </div>
            <div>
              <dt>{t('ui.spellPoints')}</dt>
              <dd className="fact-value">{hero.spellPoints}</dd>
            </div>
            <div>
              <dt>{t('ui.movementReadOnly')}</dt>
              <dd className="fact-value">
                {hero.movementLeft.toLocaleString(i18n.resolvedLanguage ?? 'en')} / {hero.movementTotal.toLocaleString(i18n.resolvedLanguage ?? 'en')}
              </dd>
            </div>
            <div>
              <dt>{t('ui.attackDefense')}</dt>
              <dd className="fact-value">
                {hero.primary.attack} / {hero.primary.defense}
              </dd>
            </div>
            <div>
              <dt>{t('ui.powerKnowledge')}</dt>
              <dd className="fact-value">
                {hero.primary.spellPower} / {hero.primary.knowledge}
              </dd>
            </div>
          </dl>
        )}
        <p className="fineprint">{t('ui.levelAndExperienceAreIndependentUpdateBothToKeep')}</p>
      </div>

      <div
        role="tabpanel"
        id="panel-spells"
        aria-labelledby="tab-spells"
        hidden={tab !== 'spells'}
        className="detail-tab"
      >
        <p className="spellbook-line">
          {t('ui.spellbook')}<strong>{hero.hasSpellbook ? t('ui.available') : t('ui.notCarried')}</strong>
          {!hero.hasSpellbook && (
            <span className="fineprint">{t('ui.valueSpellsCanBeRecordedAsLearnedButNot', { value1: ' ' })}</span>
          )}
        </p>
        {editable && edit ? (
          <SpellChecklist
            choices={edit.spellChoices}
            learnedIds={learnedIds}
            dirtySpells={dirtySpells}
            onToggle={(id, learned) => onApplyEdit({ spell: { id, learned } })}
          />
        ) : (
          <SpellList title={t('ui.learned')} spells={hero.spellsLearned} />
        )}
        <details className="secondary-details"><summary>{t('ui.availableSpellsReadOnlyValue', { value1: hero.spellsAvailable.length })}</summary>
          <SpellList title={t('ui.available')} spells={hero.spellsAvailable} />
        </details>
      </div>

      <div
        role="tabpanel"
        id="panel-skills"
        aria-labelledby="tab-skills"
        hidden={tab !== 'skills'}
        className="detail-tab"
      >
        <SkillsEditor key={hero.id}
          skills={edit?.id === hero.id ? edit.skills : hero.skills}
          choices={edit?.id === hero.id ? edit.skillChoices : []}
          readOnlyReason={edit?.id === hero.id ? edit.skillsReadOnlyReason : t('ui.loadingSkillEditingSupport')}
          dirty={dirty.has("Skills")}
          onApplyEdit={onApplyEdit} />
      </div>

      <div
        role="tabpanel"
        id="panel-army"
        aria-labelledby="tab-army"
        hidden={tab !== 'army'}
        className="detail-tab"
      >
        <ArmyEditor key={hero.id} slots={edit?.id === hero.id ? edit.army : hero.army}
          choices={edit?.id === hero.id ? edit.creatureChoices : []}
          readOnlyReason={edit?.id === hero.id ? edit.armyReadOnlyReason : t('ui.loadingArmyEditingSupport')}
          dirtyFields={dirty} onApplyEdit={onApplyEdit} />
      </div>

      <div
        role="tabpanel"
        id="panel-artifacts"
        aria-labelledby="tab-artifacts"
        hidden={tab !== 'artifacts'}
        className="detail-tab"
      >
        <div className="artifact-panels"><EquipmentEditor key={`equipment-${hero.id}`}
          target={artifactTarget?.hero === hero.id && artifactTarget.area === 'equipment' ? artifactTarget.slot : null}
          setTarget={slot => setArtifactTarget(slot === null ? null : { hero: hero.id, area: 'equipment', slot })}
          slots={edit?.id === hero.id ? edit.equipment : []}
          backpack={edit?.id === hero.id ? edit.backpack : []}
          choices={edit?.id === hero.id ? edit.artifactChoices : []}
          readOnlyReason={edit?.id === hero.id ? edit.equipmentReadOnlyReason : t('ui.loadingEquipment')}
          dirtyFields={dirty} onApplyEdit={onApplyEdit} />
        <BackpackEditor key={`backpack-${hero.id}`}
          target={artifactTarget?.hero === hero.id && artifactTarget.area === 'backpack' ? artifactTarget.slot : null}
          setTarget={slot => setArtifactTarget(slot === null ? null : { hero: hero.id, area: 'backpack', slot })}
          slots={edit?.id === hero.id ? edit.backpack : hero.backpack}
          choices={edit?.id === hero.id ? edit.artifactChoices : []}
          spells={edit?.id === hero.id ? edit.spellChoices : []}
          readOnlyReason={edit?.id === hero.id ? edit.backpackReadOnlyReason : t('ui.loadingBackpack')}
          dirtyFields={dirty} onApplyEdit={onApplyEdit} />
        </div>
        <details className="secondary-details">
          <summary>{t('ui.advancedRawData')}</summary>
          <ul className="raw-data">{hero.equipment.map((slot) =>
            <li key={slot.slot}>{slot.slot}: <code>{toHex(slot.bytes)}</code></li>
          )}</ul>
          <p>{t('ui.originalBackpackBytesPendingEditsShownAbove')}</p>
          <code className="raw-data">{toHex(hero.inventory)}</code>
        </details>
      </div>
      <details className="secondary-details">
        <summary>{t('ui.advancedDiagnostics')}</summary>
        <p className="fineprint">{t('ui.recordValuePayloadSpanValueValueStoredAttributesDo', { value1: hero.id, value2: hero.spanStart, value3: hero.spanEnd })}</p>
        {hero.notes.length > 0 && <ul className="hero-notes">
          {hero.notes.map((note, index) => <li key={`${note.kind}-${index}`}>
            <span className="diagnostic-code">{note.kind}</span> {note.message}
          </li>)}
        </ul>}
      </details>
    </article>
  );
}

/** Numeric edit field committing on blur or Enter; resets invalid input. */
function NumberField({
  label,
  value,
  min,
  max,
  dirty,
  onCommit,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  dirty: boolean;
  onCommit: (value: number) => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    setText(String(value));
  }, [value]);

  const commit = () => {
    const parsed = Number(text);
    if (!Number.isInteger(parsed) || parsed < min || parsed > max) {
      setText(String(value));
      return;
    }
    if (parsed !== value) {
      onCommit(parsed);
    }
  };

  return (
    <div className={dirty ? 'edit-dirty' : undefined}>
      <dt>
        {label}
        {dirty && <span className="dirty-dot" title={t('ui.pendingEdit')} />}
      </dt>
      <dd>
        <input
          className="edit-number"
          aria-label={label}
          type="number"
          value={text}
          min={min}
          max={max}
          onChange={(event) => setText(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.currentTarget.blur();
            }
          }}
        />
      </dd>
    </div>
  );
}

/** Searchable, dense learned-spell checklist. */
function SpellChecklist({
  choices,
  learnedIds,
  dirtySpells,
  onToggle,
}: {
  choices: SpellDto[];
  learnedIds: Set<number>;
  dirtySpells: Set<number>;
  onToggle: (id: number, learned: boolean) => void;
}) {
  const [query, setQuery] = useState('');
  const needle = query.trim().toLowerCase();
  const visible = needle
    ? choices.filter((spell) =>
        (spell.name ?? t('ui.spellValue', { value1: spell.id })).toLowerCase().includes(needle),
      )
    : choices;

  return (
    <div className="spell-group">
      <div className="spell-toolbar">
        <h4>{t('ui.learned')}</h4>
        <input
          type="search"
          placeholder={t('ui.filterSpells')}
          aria-label={t('ui.filterSpells2')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>
      <ul className="spell-checklist">
        {visible.map((spell) => (
          <li key={spell.id}>
            <label className={dirtySpells.has(spell.id) ? 'spell-dirty' : undefined}>
              <input
                type="checkbox"
                checked={learnedIds.has(spell.id)}
                onChange={(event) => onToggle(spell.id, event.target.checked)}
              />
              <SpellIcon id={spell.id} />
              {spell.name ?? t('ui.spellValue2', { value1: spell.id })}
            </label>
          </li>
        ))}
        {visible.length === 0 && <li className="fineprint">{t('ui.noSpellsMatch')}</li>}
      </ul>
    </div>
  );
}

function SpellList({ title, spells }: { title: string; spells: SpellDto[] }) {
  return (
    <div className="spell-group">
      <h4>{title}</h4>
      {spells.length > 0 ? (
        <ul className="chip-list">
          {spells.map((spell) => (
            <li key={spell.id} className="chip">
              <SpellIcon id={spell.id} />
              {spell.name ?? t('ui.spellValue2', { value1: spell.id })}
            </li>
          ))}
        </ul>
      ) : (
        <p className="fineprint">{t('ui.none')}</p>
      )}
    </div>
  );
}


/** One-line summary of what the source map says about this hero type. */
function describeMapOrigin(info: NonNullable<HeroDetailDto['map']>, encoding: LegacyEncoding): string {
  const parts: string[] = [];
  if (info.placement === 'hero' || info.placement === 'prison') {
    const level = info.level === 1 ? t('ui.underground') : t('ui.surface');
    const owner = info.owner !== null ? t('ui.valueHero', { value1: colorName(info.owner) }) : t('ui.neutralHero');
    parts.push(t('ui.valueAtValueValueValue', { value1: info.placement === 'prison' ? t('ui.inAPrison') : owner, value2: info.x, value3: info.y, value4: level }));
  } else if (info.placement === 'not-placed') {
    parts.push(t('ui.notPlacedOnTheMap'));
  } else if (info.placement === 'ambiguous') {
    parts.push(t('ui.placedMoreThanOnceOnTheMap'));
  } else {
    parts.push(t('ui.placementUnknown'));
  }
  if (info.mapName) parts.push(t('ui.mapNameValue', { value1: decodeText(info.mapName, encoding) }));
  if (info.mapPortraitId !== null) {
    parts.push(t('ui.mapPortraitValueValue', { value1: info.mapPortraitId, value2: info.portraitMatches === false ? t('ui.saveDiffers') : '' }));
  }
  return t('ui.mapStartValue', { value1: parts.join(' · ') });
}

import { ReasonText, reasonText } from '../i18n/reasons';
import { colorName } from '../i18n/labels';
import { t } from '../i18n/index';
import { useEffect, useState } from 'react';
import { getHeroPortrait } from '../api/resources';
import { Portrait } from './Portrait';
import { playerColor } from '../lib/playerColor';
import type {
  GarrisonHeroArmyDto,
  HeroRefDto,
  PortraitDto,
  TownDetailDto,
  TownEditRequest,
  TownEditResponseDto,
} from '../types/api';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';
import { ArmyEditor } from './ArmyEditor';
import { TownBuildings } from './TownBuildings';
import { TownGuild } from './TownGuild';
import { TownIcon } from './TownIcon';

const TABS = [
  { key: 'garrison', get label() { return t('ui.garrison'); } },
  { key: 'buildings', get label() { return t('ui.buildings'); } },
  { key: 'guild', get label() { return t('ui.mageGuild'); } },
] as const;
type TabKey = (typeof TABS)[number]['key'];


/**
 * Town inspection and editing (H3F-013, H3F-022). The header carries the
 * original town icon and the verified read-only facts; three tabs hold the
 * garrison (same compact editor heroes use), the buildings and the Mage
 * Guild. While a hero is garrisoned, that hero's army is the garrison: the
 * Garrison tab shows and edits those stacks through the hero's own army
 * edit and links to the hero's Army tab. Buildings and guild spells stay
 * editable. Map
 * facts are read-only reference data about the scenario start.
 */
export function TownDetail({
  town,
  encoding,
  edit,
  onApplyEdit,
  onDiscardTown,
  onSelectHero,
  onApplyHeroArmyEdit,
}: {
  town: TownDetailDto;
  encoding: LegacyEncoding;
  edit: TownEditResponseDto | null;
  onApplyEdit: (request: TownEditRequest) => void;
  onDiscardTown: () => void;
  /** Opens a hero record by ID, optionally on its Army tab. */
  onSelectHero: (heroId: number, tab?: 'army') => void;
  /** Army edit of the garrisoned hero, identified by hero record ID. */
  onApplyHeroArmyEdit: (heroId: number, request: TownEditRequest) => void;
}) {
  const [tab, setTab] = useState<TabKey>('garrison');
  const current = edit?.id === town.id ? edit : null;
  const editable = current?.editable === true;
  const dirty = new Set(current?.dirtyFields ?? []);
  const guildLevel = current?.guild?.builtLevel ?? town.mageGuildLevel;
  const dirtyTab = (key: TabKey) =>
    [...dirty].some((field) =>
      key === 'garrison'
        ? field.startsWith("Garrison") || (current?.garrisonHeroArmy?.dirtyFields.length ?? 0) > 0
        : key === 'buildings'
          ? field === 'Buildings'
          : field.startsWith("Mage Guild"));
  const level = town.z === 1 ? t('ui.underground') : t('ui.surface');

  return (
    <article className="hero-detail" aria-labelledby="town-detail-title">
      <header className="hero-detail-head" data-player-color={playerColor(town.owner)}>
        <div className="hero-detail-id">
          <TownIcon
            townType={town.townType}
            fort={town.hasFort}
            large
            label={town.townTypeName ?? t('ui.townTypeValue', { value1: town.townType })}
          />
          <div>
            <p className="eyebrow">{town.townTypeName ?? t('ui.townTypeValue', { value1: town.townType })}</p>
            <h2 id="town-detail-title">{decodeText(town.name, encoding)}</h2>
            <p className="hero-map-origin">{t('ui.positionValueValueValue', { value1: town.x, value2: town.y, value3: level })}</p>
          </div>
        </div>
        <div className="hero-badges">
          <span className={`owner owner-${town.owner.kind}`}>{colorName(town.owner.player)}</span>
          {town.confidence === 'low' && <span className="flag flag-warn">{t('ui.lowConfidence')}</span>}
          {editable ? (
            <span className="badge-editable">{t('ui.editable')}</span>
          ) : (
            <span className="badge-readonly" title={reasonText(edit?.readOnlyReason)}>
              {t('ui.readOnly')}</span>
          )}
          {editable && dirty.size > 0 && (
            <button type="button" className="revert-button" onClick={onDiscardTown}>{t('ui.revertChanges')}</button>
          )}
        </div>
      </header>

      {edit && !edit.editable && edit.readOnlyReason && (
        <p className="fineprint edit-blocked">{<ReasonText reason={edit.readOnlyReason} />}</p>
      )}

      <dl className="facts general-grid">
        <div>
          <dt>{t('ui.coordinatesReadOnly')}</dt>
          <dd className="fact-value">
            {town.x}, {town.y}, {town.z} ({level})
          </dd>
        </div>
        <div>
          <dt>{t('ui.mageGuild2')}</dt>
          <dd className="fact-value">{t('ui.levelValue', { value1: guildLevel })}</dd>
        </div>
      </dl>

      {(town.garrisonedHero || town.visitingHero) && (
        <div className="town-heroes">
          {town.garrisonedHero && (
            <p className="town-hero-row">
              <strong>{t('ui.garrisonedHero')}</strong>{' '}
              <button
                type="button"
                className="link-button"
                title={t('ui.openThisHeroItsArmyIsThisTownS')}
                onClick={() => onSelectHero(town.garrisonedHero!.id)}
              >
                {decodeText(town.garrisonedHero.name, encoding)}
              </button>
              <span className="fineprint">{t('ui.valueThisHeroSArmyIsTheGarrison', { value1: ' ' })}</span>
            </p>
          )}
          {town.visitingHero && (
            <p className="town-hero-row">
              <strong>{t('ui.visitingHero')}</strong>{' '}
              <button
                type="button"
                className="link-button"
                title={t('ui.openThisHero')}
                onClick={() => onSelectHero(town.visitingHero!.id)}
              >
                {decodeText(town.visitingHero.name, encoding)}
              </button>
            </p>
          )}
        </div>
      )}

      <div className="detail-tabs" role="tablist" aria-label={t('ui.townSections')}>
        {TABS.map((entry) => (
          <button
            key={entry.key}
            type="button"
            role="tab"
            id={`town-tab-${entry.key}`}
            aria-controls={`town-panel-${entry.key}`}
            aria-selected={tab === entry.key}
            className={tab === entry.key ? 'detail-tab-active' : undefined}
            onClick={() => setTab(entry.key)}
          >
            {entry.label}
            {dirtyTab(entry.key) && <span className="tab-dirty" aria-label={t('ui.hasPendingEdits')}> •</span>}
          </button>
        ))}
      </div>

      <section
        role="tabpanel"
        id="town-panel-garrison"
        aria-labelledby="town-tab-garrison"
        hidden={tab !== 'garrison'}
        className="detail-tab"
      >
        {current?.garrisonHeroArmy && current.garrisonedHero ? (
          <GarrisonHeroArmy
            key={`hero-${current.garrisonHeroArmy.heroId}`}
            hero={current.garrisonedHero}
            army={current.garrisonHeroArmy}
            choices={current.creatureChoices}
            heroName={decodeText(current.garrisonedHero.name, encoding)}
            onSelectHero={onSelectHero}
            onApplyHeroArmyEdit={onApplyHeroArmyEdit}
          />
        ) : (
          <ArmyEditor
            key={town.id}
            slots={edit?.id === town.id ? edit.garrison : town.garrison}
            choices={edit?.id === town.id ? edit.creatureChoices : []}
            readOnlyReason={
              edit?.id === town.id
                ? edit.garrisonReadOnlyReason
                : t('ui.loadingGarrisonEditingSupport')
            }
            dirtyFields={dirty}
            dirtyPrefix="Garrison slot"
            onApplyEdit={onApplyEdit}
          />
        )}
      </section>

      <section
        role="tabpanel"
        id="town-panel-buildings"
        aria-labelledby="town-tab-buildings"
        hidden={tab !== 'buildings'}
        className="detail-tab"
      >
        {current ? (
          <TownBuildings buildings={current.buildings} onApplyEdit={onApplyEdit} />
        ) : (
          <p className="fineprint">{t('ui.loadingBuildingState')}</p>
        )}
      </section>

      <section
        role="tabpanel"
        id="town-panel-guild"
        aria-labelledby="town-tab-guild"
        hidden={tab !== 'guild'}
        className="detail-tab"
      >
        {current ? (
          <TownGuild guild={current.guild} onApplyEdit={onApplyEdit} />
        ) : (
          <p className="fineprint">{t('ui.loadingMageGuildState')}</p>
        )}
      </section>

      {town.map && (
        <p className="hero-map-origin"
          title={t('ui.fromTheSourceMapH3mHowThisTownStarts')}>
          {describeTownMapOrigin(town, encoding)}
        </p>
      )}

      <details className="secondary-details">
        <summary>{t('ui.advancedDiagnostics')}</summary>
        <p className="fineprint">{t('ui.recordValuePayloadSpanValueValueRecruitsExternalDwellings', { value1: town.id, value2: town.spanStart, value3: town.spanEnd })}</p>
        {town.notes.length > 0 && (
          <ul className="hero-notes">
            {town.notes.map((note, index) => (
              <li key={`${note.kind}-${index}`}>
                <span className="diagnostic-code">{note.kind}</span> {note.message}
              </li>
            ))}
          </ul>
        )}
      </details>
    </article>
  );
}

/**
 * Garrison of a town with a garrisoned hero: the hero's portrait and name,
 * a jump to the hero's Army tab, and the hero's own stacks in the shared
 * army editor. Edits go to the hero record by ID.
 */
function GarrisonHeroArmy({
  hero,
  army,
  choices,
  heroName,
  onSelectHero,
  onApplyHeroArmyEdit,
}: {
  hero: HeroRefDto;
  army: GarrisonHeroArmyDto;
  choices: TownEditResponseDto['creatureChoices'];
  heroName: string;
  onSelectHero: (heroId: number, tab?: 'army') => void;
  onApplyHeroArmyEdit: (heroId: number, request: TownEditRequest) => void;
}) {
  const [portrait, setPortrait] = useState<PortraitDto | null>(null);
  useEffect(() => {
    let active = true;
    setPortrait(null);
    getHeroPortrait(hero.id, 'small').then(
      (result) => { if (active) setPortrait(result); },
      () => { if (active) setPortrait(null); },
    );
    return () => { active = false; };
  }, [hero.id]);
  return (
    <>
      <div className="garrison-hero">
        <Portrait portrait={portrait} size="small" alt={t('ui.valuePortrait', { value1: heroName })} />
        <div className="garrison-hero-text">
          <strong>{heroName}</strong>
          <span className="fineprint">{t('ui.garrisonedHeroWhileInsideThisHeroSArmyIs')}</span>
        </div>
        <button type="button" className="revert-button" onClick={() => onSelectHero(hero.id, 'army')}>{t('ui.editHeroArmy')}</button>
      </div>
      <ArmyEditor
        slots={army.slots}
        choices={choices}
        readOnlyReason={army.readOnlyReason}
        dirtyFields={new Set(army.dirtyFields)}
        onApplyEdit={(request) => onApplyHeroArmyEdit(army.heroId, request)}
      />
    </>
  );
}

/** One-line summary of what the source map says about this town's position. */
function describeTownMapOrigin(
  town: TownDetailDto,
  encoding: LegacyEncoding,
): string {
  const info = town.map;
  if (!info) return '';
  const parts: string[] = [];
  if (info.ambiguous) {
    parts.push(t('ui.severalMapTownsShareThisPosition'));
  } else if (info.matched) {
    parts.push(t('ui.matchesTheMapTownObject'));
  } else {
    parts.push(t('ui.noMapTownObjectAtThisPosition'));
  }
  if (info.mapName) {
    parts.push(
      t('ui.mapNameValueValue', { value1: decodeText(info.mapName, encoding), value2: info.nameMatches ? '' : t('ui.saveDiffers') }),
    );
  }
  if (info.mapFaction !== null) {
    parts.push(t('ui.mapFactionValueValue', { value1: info.mapFaction, value2: info.factionMatches === false ? t('ui.saveDiffers') : '' }));
  }
  if (info.mapOwner !== null) {
    const owner = colorName(info.mapOwner) ?? t('ui.playerValue', { value1: info.mapOwner });
    parts.push(t('ui.startsAsValueValue', { value1: owner, value2: info.ownerMatches === false ? t('ui.saveDiffers') : '' }));
  }
  if (info.mapHasGarrison) parts.push(t('ui.mapGaveAStartingGarrison'));
  return t('ui.mapStartValue', { value1: parts.join(' · ') });
}

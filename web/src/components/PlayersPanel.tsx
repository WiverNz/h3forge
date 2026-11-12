import { ReasonText } from '../i18n/reasons';
import { colorName, resourceName } from '../i18n/labels';
import { t, i18n } from '../i18n/index';
import { useEffect, useRef, useState } from 'react';
import { getKeymasterIcon, getResourceIcon, getPlayerFlag, getPlayerStartingPortrait } from '../api/resources';
import type { PlayerDto, PortraitDto } from '../types/api';
import { playerColor } from '../lib/playerColor';
import { TownIcon } from './TownIcon';
import { Portrait } from './Portrait';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';

const TABS = ['Resources', 'General', 'Exploration'] as const;
type PlayerTab = typeof TABS[number];
const STATUS = { active: 'status.active', defeated: 'status.defeated', unused: 'status.unused', unknown: 'status.unknown' } as const;

function eligibility(value: string | null): string {
  const keys = { 'Human or Computer': 'messages.either', Human: 'messages.human', Computer: 'messages.computer', 'Not playable': 'messages.notPlayable' } as const;
  return value && Object.hasOwn(keys, value) ? t(keys[value as keyof typeof keys]) : t('ui.eligibilityUnknown');
}
const TENT_KEYS = ['lightBlue', 'green', 'red', 'darkBlue', 'brown', 'purple', 'white', 'black'] as const;
const NAMES = ['Wood', 'Mercury', 'Ore', 'Sulfur', 'Crystal', 'Gems', 'Gold'];

function ResourceInput({ value, original, name, disabled, dirty, onChange }: {
  value: number; original: number; name: string; disabled: boolean; dirty: boolean;
  onChange: (value: number) => void;
}) {
  const [text, setText] = useState(String(value));
  const [error, setError] = useState('');
  const focused = useRef(false);
  useEffect(() => { if (!focused.current) { setText(String(value)); setError(''); } }, [value]);
  return <div className="player-resource-input">
    <input aria-label={name} aria-invalid={!!error} type="text" inputMode="numeric"
      className={dirty ? 'field-dirty' : ''} value={text} disabled={disabled}
      title={t('ui.originalValue', { value1: original.toLocaleString(i18n.resolvedLanguage ?? 'en') })} onFocus={() => { focused.current = true; }}
      onBlur={() => { focused.current = false; }}
      onChange={(event) => {
        const next = event.target.value; setText(next);
        if (!/^\d+$/.test(next) || !Number.isSafeInteger(Number(next)) || Number(next) > 2147483647) {
          setError('invalid'); return;
        }
        setError(''); onChange(Number(next));
      }} />
    {dirty && <span className="resource-dirty" title={t('ui.originalValue', { value1: original })}>{t('ui.changed')}</span>}
    {error && <small className="error" role="alert">{t('ui.valueLastValidValueRemainsPending', { value1: t('ui.enterAWholeNumberFrom0To2147') })}</small>}
  </div>;
}

export function PlayersPanel({ players, selectedId, onSelect, onEdit, onTentEdit, onRevert, epoch, artworkVersion, busy, encoding, mapLinkEpoch }: {
  encoding: LegacyEncoding; mapLinkEpoch: number;
  players: PlayerDto[]; selectedId: number; onSelect: (id: number) => void;
  onTentEdit: (id: number, color: number, visited: boolean) => void;
  onEdit: (id: number, resource: number, value: number) => void;
  onRevert: (id: number) => void; epoch: number; artworkVersion: string; busy: boolean;
}) {
  const [tab, setTab] = useState<PlayerTab>('Resources');
  const tabButtons = useRef<(HTMLButtonElement | null)[]>([]);
  useEffect(() => { setTab('Resources'); }, [selectedId]);
  const [tentIcons, setTentIcons] = useState<(PortraitDto | null)[]>([]);
  const [flags, setFlags] = useState<(PortraitDto | null)[]>([]);
  const [startingPortrait, setStartingPortrait] = useState<{ id: number; portrait: PortraitDto } | null>(null);
  const [icons, setIcons] = useState<(PortraitDto | null)[]>([]);
  useEffect(() => {
    let active = true; setIcons([]); setFlags([]); setTentIcons([]);
    Promise.all(TENT_KEYS.map((_, color) => getKeymasterIcon(color).catch(() => null))).then(result => { if (active) setTentIcons(result); });
    Promise.all(Array.from({ length: 8 }, (_, color) => getPlayerFlag(color).catch(() => null))).then((result) => { if (active) setFlags(result); });
    Promise.all(NAMES.map((_, id) => getResourceIcon(id).catch(() => null))).then((result) => {
      if (active) setIcons(result);
    });
    return () => { active = false; };
  }, [artworkVersion]);
  const player = players.find((p) => p.id === selectedId);
  const startingPortraitId = player?.details.startingHero?.portraitId;
  useEffect(() => {
    let active = true; setStartingPortrait(null);
    if (startingPortraitId !== null && startingPortraitId !== undefined) {
      getPlayerStartingPortrait(selectedId).then((portrait) => {
        if (active) setStartingPortrait({ id: selectedId, portrait });
      }, () => {});
    }
    return () => { active = false; };
  }, [selectedId, startingPortraitId, artworkVersion, mapLinkEpoch]);
  const colorNames = (colors: number[] | null) => colors === null ? t('ui.unknown') : colors.length === 0 ? t('ui.none2') : colors.map((color) => colorName(color)).join(', ');
  const flag = (color: number, large = false) => flags[color]?.pngBase64
    ? <img className={large ? 'player-banner' : 'player-list-flag'} src={`data:image/png;base64,${flags[color]?.pngBase64}`} alt={t('ui.valueBanner', { value1: colorName(color) })} width={large ? 58 : 24} height={large ? 64 : 27} />
    : <span className={large ? 'player-banner' : 'player-list-flag'} title={t('ui.originalBannerUnavailable')}>?</span>;
  if (!players.length) return <p className="scope-note column-placeholder">{t('ui.playerResourcesAreReadOnlyNoUnambiguousHota44')}</p>;
  return <>
    <section className="hero-list" aria-label={t('ui.players')}>
      <div className="hero-list-head"><h2>{t('ui.players')}</h2><span>{players.length} {t('ui.slots')}</span></div>
      <ul className="hero-rows" role="listbox" aria-label={t('ui.playerRecords')}>
        {players.map((p) => <li key={p.id}><button type="button" role="option"
          aria-selected={p.id === selectedId} data-player-color={playerColor({ kind: 'player', player: p.color })}
          className={`hero-row ${p.id === selectedId ? 'selected' : ''} ${p.status === 'unknown' ? 'uncertain' : ''}`}
          onClick={() => { setTab('Resources'); onSelect(p.id); }}>
          {flag(p.color)}
          <span className="hero-row-text"><span className="hero-row-name">{colorName(p.color)}{(p.dirtyResources.length > 0 || p.dirtyTents.length > 0) ? ' *' : ''}<span className={`player-status status-${p.status}`} title={t(p.status === 'active' ? 'messages.activeEvidence' : 'messages.unknownEvidence')}>{t(STATUS[p.status])}</span></span>
            <span className="hero-row-meta">{eligibility(p.details.eligibility)} · {p.details.team === null ? t('ui.noVerifiedTeam') : t('ui.teamValue', { value1: p.details.team })} {t('ui.map2')}</span>
            <span className="hero-row-meta">{t('ui.valueHeroesValueTowns', { value1: p.heroCount, value2: p.townCount })}</span>
          </span>
        </button></li>)}
      </ul>
    </section>
    <div className="player-editor">
      {player && <section className="player-detail" data-player-color={playerColor({ kind: 'player', player: player.color })}>
        <header className="player-heading">{flag(player.color, true)}<div><h2>{colorName(player.color)}</h2>
          <p className="scope-note"><span className={`player-status status-${player.status}`} title={t(player.status === 'active' ? 'messages.activeEvidence' : 'messages.unknownEvidence')}>{t(STATUS[player.status])}</span> · {t('ui.valueHeroesValueTowns', { value1: player.heroCount, value2: player.townCount })}</p></div>
          <button type="button" disabled={(!player.dirtyResources.length && !player.dirtyTents.length) || busy} onClick={() => onRevert(player.id)}>{t('ui.revertPlayer')}</button>
        </header>
        {player.readOnlyReason && <p className="scope-note">{<ReasonText reason={player.readOnlyReason} />}</p>}
        <div className="detail-tabs player-tabs" role="tablist" aria-label={t('ui.playerDetails')}>
          {TABS.map((label, index) => <button key={label} id={`tab-${label}`} type="button"
            ref={(element) => { tabButtons.current[index] = element; }} role="tab"
            aria-selected={tab === label} aria-controls={`player-${label}`} tabIndex={tab === label ? 0 : -1}
            className={tab === label ? 'detail-tab-active' : ''} onClick={() => setTab(label)}
            onKeyDown={(event) => {
              const next = event.key === 'ArrowRight' ? (index + 1) % 3 : event.key === 'ArrowLeft' ? (index + 2) % 3 : event.key === 'Home' ? 0 : event.key === 'End' ? 2 : -1;
              if (next >= 0) { event.preventDefault(); setTab(TABS[next]!); tabButtons.current[next]?.focus(); }
            }}>{label === 'Resources' ? t('common.resources') : label === 'General' ? t('ui.general') : t('common.exploration')}</button>)}
        </div>
        <div className="player-tab-panels">
<section role="tabpanel" id="player-Resources" aria-labelledby="tab-Resources" hidden={tab !== 'Resources'}>
        <div className="player-resources">
          {NAMES.map((_, resource) => <div className="player-resource-row" key={`${epoch}-${player.id}-${resource}`}>
            {icons[resource]?.pngBase64 ? <img width={32} height={32} alt="" src={`data:image/png;base64,${icons[resource]?.pngBase64}`} />
              : <span className="resource-placeholder" title={t('ui.originalArtworkUnavailable')}>?</span>}
            <span>{resourceName(resource)}</span>
            <ResourceInput name={resourceName(resource)} value={player.resources[resource]!} original={player.original[resource]!}
              disabled={!!player.readOnlyReason} dirty={player.dirtyResources.includes(resource)}
              onChange={(value) => onEdit(player.id, resource, value)} />
          </div>)}
        </div>
        <p className="scope-note">{t('ui.changesArePendingImmediatelySaveWritesThemToDisk')}</p>
        </section>
        <section className="player-general" role="tabpanel" id="player-General" aria-labelledby="tab-General" hidden={tab !== 'General'}>
          <p className="scope-note">{t(player.status === 'active' ? 'messages.activeEvidence' : 'messages.unknownEvidence')}</p>
          <dl className="player-facts">
            <dt>{t('ui.currentController')}</dt><dd title={t('messages.controller')}>{player.details.controller ?? t('ui.unknownSaveFieldsUnverified')}</dd>
            <dt>{t('ui.mapEligibility')}</dt><dd>{eligibility(player.details.eligibility)}</dd>
            <dt>{t('ui.mapTeam')}</dt><dd>{player.details.team !== null ? t('ui.teamValue', { value1: player.details.team }) : player.details.mapLinked ? t('ui.noAuthoredTeam') : t('ui.unknown')}</dd>
            <dt>{t('ui.mapAllies')}</dt><dd>{colorNames(player.details.allies)}</dd>
            <dt>{t('ui.mapEnemies')}</dt><dd>{colorNames(player.details.enemies)}</dd>
          </dl>
          <p className="scope-note">{t(player.details.mapLinked ? 'messages.teamLinked' : 'messages.teamMissing')}</p>
          <div className="player-starting">
            <div className="player-start-card">
              {player.details.startingTown?.faction !== null && player.details.startingTown?.faction !== undefined
                ? <TownIcon townType={player.details.startingTown.faction} fort={false} label={player.details.startingTown.factionName} />
                : <span className="town-icon town-icon-small town-icon-placeholder" title={t('ui.startingFactionIsRandomOrUnidentified')}>?</span>}
              <div><strong>{t('ui.startingTownH3m')}</strong>
                <p>{player.details.startingTown?.name ? decodeText(player.details.startingTown.name, encoding) : player.details.startingTown?.factionName ?? t('ui.notUniquelyIdentified')}</p>
                {player.details.startingTown && <small>{player.details.startingTown.name ? `${player.details.startingTown.factionName} · ` : ''}{player.details.startingTown.x}, {player.details.startingTown.y} · {player.details.startingTown.z ? t('ui.underground') : t('ui.surface')} {t('ui.entrance')}</small>}
              </div>
            </div>
            <div className="player-start-card">
              <Portrait portrait={startingPortrait?.id === player.id ? startingPortrait.portrait : null} size="small" alt={t('ui.startingHeroPortrait')} />
              <div><strong>{t('ui.startingHeroH3m')}</strong>
                <p>{player.details.startingHero?.name ? decodeText(player.details.startingHero.name, encoding) : player.details.startingHero ? t('ui.heroTypeValue', { value1: player.details.startingHero.heroType }) : player.details.startingHeroReason}</p>
                {player.details.startingHero && <small>{player.details.startingHero.portraitSource}</small>}
              </div>
            </div>
          </div>
          <p className="scope-note">{t('ui.startingInformationDescribesTheScenarioBeforeTurnOneNot')}</p>
        </section>
        <section className="player-exploration" role="tabpanel" id="player-Exploration" aria-labelledby="tab-Exploration" hidden={tab !== 'Exploration'}>

          <section className="keymaster-tents" aria-label={t('tents.title')}>
            <h3>{t('tents.title')}</h3>
            <p className="scope-note">{t('tents.scope', { player: colorName(player.color) })}</p>
            {player.tentVisits ? <>
              <div className="tent-grid">{TENT_KEYS.map((key, color) => <label className={`tent-row ${player.dirtyTents.includes(color) ? 'tent-dirty' : ''}`} key={key}>
                {tentIcons[color]?.pngBase64 ? <img width={72} height={48} alt="" src={`data:image/png;base64,${tentIcons[color]?.pngBase64}`} /> : <span className="resource-placeholder" title={t('ui.originalArtworkUnavailable')}>?</span>}
                <span>{t(`tents.${key}`)}{player.dirtyTents.includes(color) ? ' *' : ''}</span>
                <select aria-label={`${colorName(player.color)}: ${t(`tents.${key}`)}`} disabled={busy}
                  value={player.tentVisits![color] ? 'yes' : 'no'} onChange={e => onTentEdit(player.id, color, e.target.value === 'yes')}>
                  <option value="no">{t('tents.notVisited')}</option><option value="yes">{t('tents.visited')}</option>
                </select>
              </label>)}</div>
              <button type="button" disabled={busy || !player.tentVisits.some(Boolean)} onClick={() => player.tentVisits!.forEach((visited, color) => { if (visited) onTentEdit(player.id, color, false); })}>{t('tents.clear')}</button>
            </> : <p className="scope-note">{t('tents.unavailable')}</p>}
          </section>
          <p className="scope-note">{player.details.mapSize ? `${player.details.mapSize} × ${player.details.mapSize}` : t('ui.unknownDimensions')} · {player.details.levels === 2 ? t('ui.surfaceAndUnderground') : player.details.levels === 1 ? t('ui.surfaceOnly') : t('ui.unknownLevels')}{t('ui.revealedTileCountUnverified')}</p>
          <button type="button" disabled aria-describedby="map-reveal-blocker">{t('ui.revealEntireMap')}</button>
          <p id="map-reveal-blocker" className="scope-note">{t('messages.revealMap')}</p>
          <button type="button" disabled aria-describedby="puzzle-reveal-blocker">{t('ui.revealPuzzleMapGrail')}</button>
          <p id="puzzle-reveal-blocker" className="scope-note">{t('messages.revealPuzzle')}</p>
          <p className="scope-note">{t('ui.obelisksInH3mValueVisitedGm2ValuePuzzleCompletion', { value1: player.details.obelisks?.length ?? t('ui.unknown'), value2: player.details.visitedObelisks ?? t('ui.unknown'), value3: player.details.puzzleProgress ?? t('ui.unknown'), value4: player.details.grailPlacements ?? t('ui.unknown') })}</p>
          {!!player.details.obelisks?.length && <details className="player-obelisks"><summary>{t('ui.obeliskLocationsInOriginalH3mValue', { value1: player.details.obelisks.length })}</summary>
            <p className="scope-note">{t('ui.storedSpriteAnchorsAndMapObjectOrdinalsNotGm2')}</p>
            <ul>{player.details.obelisks.map((o) => <li key={o.mapObjectId}>{t('ui.objectValueValueValueValue', { value1: o.mapObjectId, value2: o.x, value3: o.y, value4: o.z ? t('ui.underground') : t('ui.surface') })}</li>)}</ul>
          </details>}
        </section>
        </div>
      </section>}
    </div>
  </>;
}

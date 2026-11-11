import { Feedback } from '../i18n/Feedback';
import { describeError, type ErrorDescriptor } from '../i18n/errors';
import { t } from '../i18n/index';
import { useCallback, useEffect, useState } from 'react';
import { chooseMapFile, getMapLink, rescanMaps, selectMapCandidate } from '../api/save';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';
import type { MapLinkDto } from '../types/api';

const STATUS_LABEL = { matched: 'mapStatus.matched', manual: 'mapStatus.manual', ambiguous: 'mapStatus.ambiguous', 'not-found': 'mapStatus.not-found', 'no-map-name': 'mapStatus.no-map-name' } as const;

const STRENGTH_TITLE = { full: 'match.full', 'name-and-dimensions': 'match.name-and-dimensions', 'name-only': 'match.name-only' } as const;

const errorText = describeError;

/**
 * Source-map row for the save Details section (H3F-021): which `.h3m` the
 * save was matched to, how, and what the map itself says. Everything shown
 * here comes from the map file and describes the scenario before turn one.
 */
export function MapLinkRow({
  saveKey,
  encoding,
  onChanged,
}: {
  /** Changes whenever a different save is opened. */
  saveKey: string;
  encoding: LegacyEncoding;
  /** Called after the linked map changes so hero details can refresh. */
  onChanged: () => void;
}) {
  const [link, setLink] = useState<MapLinkDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ErrorDescriptor | null>(null);
  const [candidate, setCandidate] = useState(0);

  useEffect(() => {
    let active = true;
    setError(null);
    getMapLink().then(
      (next) => { if (active) setLink(next); },
      (reason) => { if (active) { setLink(null); setError(errorText(reason)); } },
    );
    return () => { active = false; };
  }, [saveKey]);

  const run = useCallback(
    (action: () => Promise<MapLinkDto | null>) => {
      setBusy(true);
      setError(null);
      action().then(
        (next) => {
          setBusy(false);
          if (next) {
            setLink(next);
            setCandidate(0);
            onChanged();
          }
        },
        (reason) => {
          setBusy(false);
          setError(errorText(reason));
        },
      );
    },
    [onChanged],
  );

  const map = link?.map ?? null;
  return (
    <div className="map-link" aria-label={t('ui.sourceMap')}>
      <ul className="save-banner-facts">
        <li>
          <span className="fact-label">{t('ui.mapFile')}</span>
          {map ? (
            <span title={map.folder}>{map.fileName}</span>
          ) : (
            <span className="map-link-missing">{link ? Object.hasOwn(STATUS_LABEL, link.status) ? t(STATUS_LABEL[link.status as keyof typeof STATUS_LABEL]) : t('ui.unknown') : '…'}</span>
          )}
        </li>
        {map && link && (
          <li title={Object.hasOwn(STRENGTH_TITLE, map.matchStrength) ? t(STRENGTH_TITLE[map.matchStrength as keyof typeof STRENGTH_TITLE]) : undefined}>
            <span className={link.status === 'matched' ? 'badge-editable' : 'badge-readonly'}>
              {Object.hasOwn(STATUS_LABEL, link.status) ? t(STATUS_LABEL[link.status as keyof typeof STATUS_LABEL]) : t('ui.unknown')}
            </span>
          </li>
        )}
        <li>
          <button type="button" className="link-button" disabled={busy}
            onClick={() => run(chooseMapFile)}>
            {map ? t('ui.chooseAnother') : t('ui.chooseMap')}
          </button>
        </li>
        <li>
          <button type="button" className="link-button" disabled={busy}
            title={link ? t('ui.searchedValue', { value1: link.searched.join(' · ') || t('ui.noMapsFolderFound') }) : undefined}
            onClick={() => run(rescanMaps)}>{t('ui.rescan')}</button>
        </li>
      </ul>
      {link && !map && <p className="fineprint map-link-note">{link.message}</p>}
      {link && link.status === 'ambiguous' && link.candidates.length > 0 && (
        <label className="encoding-select">
          {t('ui.candidate')}<select value={candidate} onChange={(event) => setCandidate(Number(event.target.value))}>
            {link.candidates.map((entry) => (
              <option key={entry.index} value={entry.index} title={entry.folder}>
                {entry.fileName} - {entry.folder}
              </option>
            ))}
          </select>
          <button type="button" className="link-button" disabled={busy}
            onClick={() => run(() => selectMapCandidate(candidate))}>{t('ui.useThisMap')}</button>
        </label>
      )}
      {map && (
        <ul className="save-banner-facts map-link-facts" title={t('ui.fromTheMapFileTheScenarioBeforeTurnOne')}>
          <li><span className="fact-label">{t('ui.map')}</span>{decodeText(map.name, encoding)}</li>
          <li>
            <span className="fact-label">{t('ui.format')}</span>
            {map.format}{map.hotaRelease ? ` · ${map.hotaRelease}` : ''}
          </li>
          <li>
            <span className="fact-label">{t('ui.size')}</span>
            {map.size}×{map.size}{map.hasUnderground ? t('ui.2Levels') : ''}
          </li>
          <li>
            <span className="fact-label">{t('ui.players')}</span>
            {map.playablePlayers} ({map.humanPlayers} {t('ui.human')}</li>
          {map.towns !== null && (
            <li><span className="fact-label">{t('ui.towns')}</span>{map.towns}</li>
          )}
          {map.placedHeroes !== null && (
            <li>
              <span className="fact-label">{t('ui.startHeroes')}</span>
              {map.placedHeroes}{map.randomHeroes ? t('ui.valueRandom', { value1: map.randomHeroes }) : ''}
            </li>
          )}
          {map.portraitsChecked > 0 && (
            <li title={t('ui.heroTypesWhosePortraitTheMapSetsExplicitlyCompared')}>
              <span className="fact-label">{t('ui.portraitIds')}</span>
              {map.portraitsConsistent}/{map.portraitsChecked} {t('ui.agree')}</li>
          )}
        </ul>
      )}
      {map?.bodyError && (
        <p className="fineprint map-link-note">{t('ui.headerOnlyHeroAndTownPlacementUnavailableValue', { value1: map.bodyError })}</p>
      )}
      {error && <p className="error map-link-note" role="alert">{<Feedback message={error} />}</p>}
    </div>
  );
}

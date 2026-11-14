import { t } from '../i18n/index';
import { useEffect, useState } from 'react';
import type { SaveSummaryDto } from '../types/api';
import { LEGACY_ENCODING_OPTIONS, decodeText, type LegacyEncoding } from '../lib/legacyText';
import { MapLinkRow } from './MapLinkRow';

/**
 * Compact save summary: a single dense row with map name, game version and
 * hero counts, plus a Details toggle revealing the full description, name
 * encoding, editing compatibility and diagnostics. The filename lives in the
 * native window title instead. Details collapse whenever a save is opened.
 */
export function SaveBanner({
  summary,
  encoding,
  onEncodingChange,
  onMapLinkChanged,
}: {
  summary: SaveSummaryDto;
  encoding: LegacyEncoding;
  onEncodingChange: (encoding: LegacyEncoding) => void;
  /** Called after the user links a different source map. */
  onMapLinkChanged: () => void;
}) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  useEffect(() => {
    setDetailsOpen(false);
  }, [summary]);
  const mapName = summary.mapName ? decodeText(summary.mapName, encoding) : null;
  const mapDescription = summary.mapDescription
    ? decodeText(summary.mapDescription, encoding)
    : null;
  const editable = summary.heroesSupported;

  return (
    <section className="save-banner" aria-labelledby="save-banner-title">
      <div className="save-banner-row">
        <h1 id="save-banner-title" className="save-banner-name" title={mapName ?? t('ui.unnamedMap')}>
          {mapName ?? t('ui.unnamedMap')}
        </h1>
        <ul className="save-banner-facts">
          <li>
            <span className="fact-label">{t('ui.version')}</span>
            {summary.familyTitle ?? t('ui.unrecognized')} {summary.major}/{summary.minor}
          </li>
          <li>
            <span className="fact-label">{t('ui.heroes')}</span>
            {t('messages.heroCounts', { located: summary.heroCount, confirmed: summary.confirmedCount })}</li>
        </ul>
        <button
          type="button"
          className="link-button save-banner-toggle"
          aria-expanded={detailsOpen}
          onClick={() => setDetailsOpen((open) => !open)}
        >
          {detailsOpen ? t('ui.hideDetails') : t('ui.details')}
        </button>
      </div>
      {summary.diagnostics.some(d => d.code === 'native-gzip-crc-mismatch') && (
        <p className="scope-note" role="status">{t('teams.nativeCrc')}</p>
      )}
      {detailsOpen && (
        <div className="save-banner-details">
          <p className="save-banner-desc">{mapDescription ?? t('ui.noMapDescriptionRecorded')}</p>
          <ul className="save-banner-facts">
            <li>
              <span className="fact-label">{t('ui.editing')}</span>
              <span className={editable ? 'badge-editable' : 'badge-readonly'}>
                {editable ? t('ui.heroEditing') : t('ui.readOnlyLayout')}
              </span>
            </li>
            <li>
              <label className="encoding-select">
                {t('ui.names')}<select
                  value={encoding}
                  onChange={(event) =>
                    onEncodingChange(event.target.value as LegacyEncoding)
                  }
                >
                  {LEGACY_ENCODING_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
            </li>
          </ul>
          <MapLinkRow
            saveKey={`${summary.fileName}:${summary.heroCount}`}
            encoding={encoding}
            onChanged={onMapLinkChanged}
          />
          {summary.diagnostics.length > 0 && (
            <details className="banner-diagnostics">
              <summary>{t('ui.diagnosticsValue', { value1: summary.diagnostics.length })}</summary>
              <ul>
                {summary.diagnostics.map((diagnostic) => (
                  <li key={diagnostic.code}>
                    <span className="diagnostic-code">{diagnostic.code}</span>{' '}
                    {diagnostic.message}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
      )}
    </section>
  );
}

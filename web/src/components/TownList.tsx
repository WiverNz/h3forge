import { colorName } from '../i18n/labels';
import { t } from '../i18n/index';
import { playerColor } from '../lib/playerColor';
import { useMemo, useState } from 'react';
import type { TownListItemDto } from '../types/api';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';
import { CreaturePortrait } from './CreaturePortrait';
import { armyPreview } from '../lib/armyPreview';
import { TownIcon } from './TownIcon';

/**
 * Searchable town list. Records are identified by opaque ID, never by name;
 * duplicate names and low-confidence candidates are visibly flagged, and
 * towns whose garrison belongs to a garrisoned hero carry an explicit flag.
 */
export function TownList({
  towns,
  encoding,
  selectedId,
  onSelect,
}: {
  towns: TownListItemDto[];
  encoding: LegacyEncoding;
  selectedId: number | null;
  onSelect: (id: number) => void;
}) {
  const [query, setQuery] = useState('');
  const [occupiedOnly, setOccupiedOnly] = useState(false);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return towns.filter((town) => {
      if (occupiedOnly && !town.hasGarrisonHero) return false;
      if (!needle) return true;
      const typeName = town.townTypeName ?? '';
      return (
        decodeText(town.name, encoding).toLowerCase().includes(needle) ||
        typeName.toLowerCase().includes(needle)
      );
    });
  }, [towns, query, occupiedOnly, encoding]);

  return (
    <section className="hero-list" aria-labelledby="town-list-title">
      <div className="hero-list-head">
        <h2 id="town-list-title">{t('ui.towns')}</h2>
        <span className="hero-list-count">
          {visible.length} {' '}{t('ui.of')}{' '}{towns.length}
        </span>
      </div>
      <div className="hero-list-filters">
        <input
          type="search"
          placeholder={t('ui.searchTowns')}
          aria-label={t('ui.searchTownsByNameOrType')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <label className="owned-filter">
          <input
            type="checkbox"
            checked={occupiedOnly}
            onChange={(event) => setOccupiedOnly(event.target.checked)}
          />
          {t('ui.heroInside')}</label>
      </div>
      <ul className="hero-rows" role="listbox" aria-label={t('ui.townRecords')}>
        {visible.map((town) => {
          const army = armyPreview(town.garrisonCreatureIds);
          return (
            <li key={town.id}>
              <button
                type="button"
                data-player-color={playerColor(town.owner)}
                role="option"
                aria-selected={town.id === selectedId}
                className={[
                  'hero-row',
                  town.id === selectedId ? 'selected' : '',
                  town.confidence === 'low' ? 'uncertain' : '',
                ]
                  .filter(Boolean)
                  .join(' ')}
                onClick={() => onSelect(town.id)}
              >
                <TownIcon
                  townType={town.townType}
                  fort={town.hasFort}
                  label={town.townTypeName ?? t('ui.townTypeValue', { value1: town.townType })}
                />
                <span className="hero-row-text">
                  <span className="hero-row-heading">
                    <span className="hero-row-name">
                      <span className="hero-row-label" title={decodeText(town.name, encoding)}>
                        {decodeText(town.name, encoding)}
                      </span>
                      {town.duplicateName && (
                        <span className="flag" title={t('ui.anotherRecordHasIdenticalNameBytes')}>
                          {t('ui.duplicate')}</span>
                      )}
                      {town.confidence === 'low' && (
                        <span className="flag flag-warn" title={t('ui.failedASecondaryStructuralCheck')}>
                          {t('ui.candidate2')}</span>
                      )}
                    </span>
                    {army.icons.length > 0 && (
                      <span className="hero-army-preview" aria-label={t('ui.valueGarrisonStacks', { value1: town.garrisonCreatureIds.length })}>
                        {army.icons.map((id, index) => (
                          <CreaturePortrait key={`${index}-${id}`} id={id} compact />
                        ))}
                        {army.remaining > 0 && (
                          <span className="hero-army-more" title={t('ui.valueMoreStacks', { value1: army.remaining })}>
                            +{army.remaining}
                          </span>
                        )}
                      </span>
                    )}
                  </span>
                  <span className="hero-row-meta">
                    <span className={`owner owner-${town.owner.kind}`}>{colorName(town.owner.player)}</span>
                    <span>{town.townTypeName ?? t('ui.typeValue', { value1: town.townType })}</span>
                    {town.hasGarrisonHero && <span title={t('ui.aGarrisonedHeroOwnsTheGarrison')}>{t('ui.heroInside2')}</span>}
                    {town.hasVisitingHero && <span title={t('ui.aVisitingHeroStandsAtTheTown')}>{t('ui.visitor')}</span>}
                  </span>
                </span>
              </button>
            </li>
          );
        })}
        {visible.length === 0 && <li className="hero-rows-empty">{t('ui.noTownsMatch')}</li>}
      </ul>
    </section>
  );
}

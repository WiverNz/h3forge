import { ListControls, OwnerFacet, Facet, useListSort } from './ListControls';
import { sortList, matchesFacet, ownerKey, ownerValue } from '../lib/listView';
import { colorName } from '../i18n/labels';
import { t } from '../i18n/index';
import { playerColor } from '../lib/playerColor';
import { useState } from 'react';
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

  const [owners, setOwners] = useState<string[]>([]);
  const [factions, setFactions] = useState<string[]>([]);
  const { sort, changeSort, locale } = useListSort('towns', ['original', 'name', 'owner', 'faction']);
  const clear = () => { setOccupiedOnly(false); setOwners([]); setFactions([]); };
  const visible = sortList(towns.filter((town) => {
    if (occupiedOnly && !town.hasGarrisonHero) return false;
    if (!matchesFacet(owners, ownerKey(town.owner))) return false;
    if (!matchesFacet(factions, town.townTypeName === null ? 'unknown' : String(town.townType))) return false;
    const needle = query.trim().toLocaleLowerCase(locale);
    return decodeText(town.name, encoding).toLocaleLowerCase(locale).includes(needle) || (town.townTypeName ?? '').toLocaleLowerCase(locale).includes(needle);
  }), sort, locale, row => {
    switch (sort.key) {
      case 'name': return decodeText(row.name, encoding) || null;
      case 'owner': return ownerValue(ownerKey(row.owner));
      case 'faction': return row.townTypeName;
      default: return null;
    }
  });

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
      <ListControls sort={sort} onSort={changeSort} options={(['original', 'name', 'owner', 'faction'] as const).map(value => ({ value, label: t(`lists.${value}`) }))}
        active={Number(occupiedOnly) + owners.length + factions.length} onClear={clear}
        hidden={selectedId !== null && !visible.some(row => row.id === selectedId)}>
        <OwnerFacet selected={owners} onChange={setOwners} />
        <Facet label={t('lists.faction')} selected={factions} onChange={setFactions} options={[...new Map(towns.map(town => [town.townTypeName === null ? 'unknown' : String(town.townType), { value: town.townTypeName === null ? 'unknown' : String(town.townType), label: town.townTypeName ?? t('ui.unknown') }])).values()]} />
      </ListControls>
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

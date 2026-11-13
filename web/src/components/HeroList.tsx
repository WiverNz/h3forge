import { ListControls, OwnerFacet, useListSort } from './ListControls';
import { sortList, matchesFacet, ownerKey, ownerValue } from '../lib/listView';
import { colorName } from '../i18n/labels';
import { t } from '../i18n/index';
import { playerColor } from '../lib/playerColor';
import { useState } from 'react';
import type { HeroListItemDto, PortraitDto } from '../types/api';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';
import { Portrait } from './Portrait';
import { CreaturePortrait } from './CreaturePortrait';
import { armyPreview } from '../lib/armyPreview';

/**
 * Searchable hero list. Records are identified by opaque ID, never by name;
 * duplicate names and low-confidence candidates are visibly flagged.
 */
export function HeroList({
  heroes,
  encoding,
  selectedId,
  portraits,
  onSelect,
}: {
  heroes: HeroListItemDto[];
  encoding: LegacyEncoding;
  selectedId: number | null;
  portraits: Map<number, PortraitDto>;
  onSelect: (id: number) => void;
}) {
  const [query, setQuery] = useState('');
  const [ownedOnly, setOwnedOnly] = useState(false);

  const [owners, setOwners] = useState<string[]>([]);
  const { sort, changeSort, resetSort, locale } = useListSort('heroes', ['original', 'name', 'owner', 'level']);
  const clear = () => { setQuery(''); setOwnedOnly(false); setOwners([]); };
  const visible = sortList(heroes.filter((hero) => {
    if (ownedOnly && hero.owner.kind !== 'player') return false;
    if (!matchesFacet(owners, ownerKey(hero.owner))) return false;
    const needle = query.trim().toLocaleLowerCase(locale);
    return decodeText(hero.name, encoding).toLocaleLowerCase(locale).includes(needle);
  }), sort, locale, row => {
    switch (sort.key) {
      case 'name': return decodeText(row.name, encoding) || null;
      case 'owner': return ownerValue(ownerKey(row.owner));
      case 'level': return row.level;
      default: return null;
    }
  });

  return (
    <section className="hero-list" aria-labelledby="hero-list-title">
      <div className="hero-list-head">
        <h2 id="hero-list-title">{t('ui.heroes')}</h2>
        <span className="hero-list-count">
          {visible.length} {' '}{t('ui.of')}{' '}{heroes.length}
        </span>
      </div>
      <div className="hero-list-filters">
        <input
          type="search"
          placeholder={t('ui.searchHeroes')}
          aria-label={t('ui.searchHeroesByName')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <label className="owned-filter">
          <input
            type="checkbox"
            checked={ownedOnly}
            onChange={(event) => setOwnedOnly(event.target.checked)}
          />
          {t('ui.ownedOnly')}</label>
      </div>
      <ListControls sort={sort} onSort={changeSort} options={(['original', 'name', 'owner', 'level'] as const).map(value => ({ value, label: t(`lists.${value}`) }))}
        active={Number(!!query) + Number(ownedOnly) + owners.length} onClear={clear} onReset={() => { clear(); resetSort(); }}
        hidden={selectedId !== null && !visible.some(row => row.id === selectedId)}>
        <OwnerFacet selected={owners} onChange={setOwners} />
      </ListControls>
      <ul className="hero-rows" role="listbox" aria-label={t('ui.heroRecords')}>
        {visible.map((hero) => {
          const army = armyPreview(hero.armyCreatureIds);
          return (
          <li key={hero.id}>
            <button
              type="button"
              data-player-color={playerColor(hero.owner)}
              role="option"
              aria-selected={hero.id === selectedId}
              className={[
                'hero-row',
                hero.id === selectedId ? 'selected' : '',
                hero.confidence === 'low' ? 'uncertain' : '',
              ]
                .filter(Boolean)
                .join(' ')}
              onClick={() => onSelect(hero.id)}
            >
              <Portrait portrait={portraits.get(hero.id)} size="small" />
              <span className="hero-row-text">
                <span className="hero-row-heading">
                <span className="hero-row-name">
                  <span className="hero-row-label" title={decodeText(hero.name, encoding)}>{decodeText(hero.name, encoding)}</span>
                  {hero.duplicateName && (
                    <span className="flag" title={t('ui.anotherRecordHasIdenticalNameBytes')}>
                      {t('ui.duplicate')}</span>
                  )}
                  {hero.confidence === 'low' && (
                    <span className="flag flag-warn" title={t('ui.failedASecondaryStructuralCheck')}>
                      {t('ui.candidate2')}</span>
                  )}
                </span>
                {army.icons.length > 0 && <span className="hero-army-preview" aria-label={t('ui.valueArmyStacks', { value1: hero.armyCreatureIds.length })}>
                  {army.icons.map((id, index) => <CreaturePortrait key={`${index}-${id}`} id={id} compact />)}
                  {army.remaining > 0 && <span className="hero-army-more" title={t('ui.valueMoreStacks', { value1: army.remaining })}>+{army.remaining}</span>}
                </span>}
                </span>
                <span className="hero-row-meta">
                  <span className={`owner owner-${hero.owner.kind}`}>{colorName(hero.owner.player)}</span>
                  <span>{t('ui.lvValue', { value1: hero.level })}</span>
                  {hero.hasSpellbook && <span title={t('ui.carriesASpellbook')}>{t('ui.spellbook2')}</span>}
                </span>
              </span>
            </button>
          </li>
          );
        })}
        {visible.length === 0 && <li className="hero-rows-empty">{t('ui.noRecordsMatch')}</li>}
      </ul>
    </section>
  );
}

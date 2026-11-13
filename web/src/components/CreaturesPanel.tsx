import { ListControls, OwnerFacet, Facet, useListSort } from './ListControls';
import { sortList, matchesFacet, creatureOwner, creatureName, creatureQuantity, matchesCreatureType, selectedRecord } from '../lib/listView';
import { useEffect, useRef, useState } from 'react';
import { applyNeutralEdit, discardNeutralEdits, listNeutralArmies } from '../api/save';
import { t } from '../i18n/index';
import { colorName } from '../i18n/labels';
import { describeError } from '../i18n/errors';
import { Feedback, type FeedbackMessage } from '../i18n/Feedback';
import type { NeutralArmyDto, NeutralListDto } from '../types/api';
import { ArmyEditor } from './ArmyEditor';
import { CreaturePortrait } from './CreaturePortrait';

export function CreaturesPanel({ epoch, onChanged, onBusy }: {
  epoch: string; onChanged: () => Promise<void>; onBusy: (delta: number) => void;
}) {
  const [data, setData] = useState<NeutralListDto | null>(null);
  const [error, setError] = useState<FeedbackMessage | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [query, setQuery] = useState('');
  const [kind, setKind] = useState<'all' | 'wandering' | 'garrison'>('wandering');
  const [level, setLevel] = useState('all');
  const [owners, setOwners] = useState<string[]>([]);
  const [types, setTypes] = useState<string[]>([]);
  const { sort, changeSort, locale } = useListSort('creatures', ['original', 'name', 'quantity', 'coordinates']);
  const clear = () => { setKind('all'); setLevel('all'); setOwners([]); setTypes([]); };
  const [requests, setRequests] = useState(0);
  const generation = useRef(0);
  const queue = useRef(Promise.resolve());
  useEffect(() => {
    const token = ++generation.current;
    setError(null);
    listNeutralArmies().then(result => {
      if (generation.current === token) { setData(result); setSelected(id => selectedRecord(result.groups, id)?.id ?? null); }
    }, cause => { if (generation.current === token) setError(describeError(cause)); });
    return () => { generation.current++; };
  }, [epoch]);
  const mutate = (action: () => Promise<NeutralListDto>) => {
    const token = generation.current;
    setRequests(n => n + 1); onBusy(1); setError(null);
    queue.current = queue.current.then(async () => {
    try {
      const result = await action();
      if (generation.current === token) { setData(result); setSelected(id => selectedRecord(result.groups, id)?.id ?? null); }
      await onChanged();
    } catch (cause) {
      if (generation.current === token) setError(describeError(cause));
    } finally {
      setRequests(n => n - 1); onBusy(-1);
    }
    });
  };
  const title = (g: NeutralArmyDto) => g.kind === 'wandering' ? g.name ?? `${t('ui.creature')} #${g.creatureId}` : `${t('neutral.garrisons')} #${g.id}`;
  const visible = sortList((data?.groups ?? []).filter(g => {
    const text = `${title(g)} ${g.id} ${g.position.join(', ')} ${g.army?.map(s => s.name ?? s.creatureId).join(' ') ?? ''}`.toLowerCase();
    return (kind === 'all' || g.kind === kind) && matchesFacet(owners, creatureOwner(g)) && matchesCreatureType(g, types) && (level === 'all' || String(g.position[2]) === level)
      && query.toLowerCase().split(/\s+/).every(part => text.includes(part));
  }), sort, locale, g => {
    if (sort.key === 'name') return creatureName(g);
    if (sort.key === 'quantity') return creatureQuantity(g);
    if (sort.key === 'coordinates') return [g.position[2], g.position[1], g.position[0]];
    return null;
  });
  const current = data?.groups.find(g => g.id === selected);
  const typeOptions = [...new Map((data?.groups ?? []).flatMap(g => g.army?.filter(slot => !slot.empty && slot.creatureId !== null)
    .map(slot => [String(slot.creatureId), { value: String(slot.creatureId), label: slot.name ?? `#${slot.creatureId}` }] as const) ?? [])).values()];
  typeOptions.sort((a, b) => new Intl.Collator(locale).compare(a.label, b.label));
  return <>
    <section className="hero-list neutral-list" aria-label={t('neutral.creatures')}>
      <div className="neutral-categories" aria-label={t('neutral.objectType')}>
        {(['all', 'wandering', 'garrison'] as const).map(category => <button type="button" key={category} aria-pressed={kind === category} onClick={() => setKind(category)}>
          {category === 'all' ? t('lists.allCategories') : category === 'wandering' ? t('neutral.wandering') : t('neutral.garrisons')} <span>{data?.groups.filter(g => category === 'all' || g.kind === category).length ?? 0}</span>
        </button>)}
      </div>
      <div className="hero-list-filters">
        <input type="search" aria-label={t('neutral.search')} placeholder={t('neutral.search')} value={query} onChange={e => setQuery(e.target.value)} />
      </div>
      <div className="neutral-filter-count">{visible.length} / {data?.groups.length ?? 0}</div>
      <ListControls sort={sort} onSort={changeSort} options={(['original', 'name', 'quantity', 'coordinates'] as const).map(value => ({ value, label: t(`lists.${value}`) }))}
        active={Number(kind !== 'all') + Number(level !== 'all') + owners.length + types.length}
        onClear={clear} hidden={!!current && !visible.some(g => g.id === current.id)}>
        <OwnerFacet selected={owners} onChange={setOwners} />
        <Facet label={t('lists.creature')} selected={types} onChange={setTypes} searchable options={[
          ...typeOptions, { value: 'empty', label: t('ui.empty') }, { value: 'unknown', label: t('ui.unknown') },
        ]} />
        <label className="list-level">{t('neutral.level')}<select aria-label={t('neutral.level')} value={level} onChange={e => setLevel(e.target.value)}>
          <option value="all">{t('neutral.allLevels')}</option><option value="0">{t('neutral.surface')}</option><option value="1">{t('neutral.underground')}</option>
        </select></label>
      </ListControls>
      <ul className="hero-rows" aria-label={t('neutral.creatures')}>
        {visible.map(g => <li key={g.id}><button type="button" className={`hero-row${current?.id === g.id ? ' selected' : ''}`} aria-pressed={current?.id === g.id} onClick={() => setSelected(g.id)}>
          <CreaturePortrait id={g.creatureId ?? g.army?.find(s => !s.empty)?.creatureId ?? null} compact />
          <span className="neutral-row-text"><strong title={title(g)}>{title(g)}</strong><small title={g.army ? g.army.filter(s => !s.empty).map(s => `${s.count} ${s.name ?? `#${s.creatureId}`}`).join(', ') : t('neutral.quantityUnknown')}>
            {g.army ? g.army.filter(s => !s.empty).map(s => `${s.count} ${s.name ?? `#${s.creatureId}`}`).join(', ') || t('ui.empty') : t('neutral.quantityUnknown')}</small></span>
          <small className="neutral-row-position" title={g.position[2] === 0 ? t('neutral.surface') : t('neutral.underground')}>{g.position.join(' / ')}</small>
        </button></li>)}
        {visible.length === 0 && <li className="hero-rows-empty">{t('ui.noRecordsMatch')}</li>}
      </ul>
    </section>
    <div className="editor-scroll neutral-editor">
      {error && <p role="alert" className="error"><Feedback message={error} /></p>}
      {!data && !error && <p>{t('neutral.loading')}</p>}
      {data?.unavailableReason && <p>{data.unavailableReason}</p>}
      {current && <section className="hero-detail neutral-detail" inert={requests > 0}>
        <div className="hero-detail-head"><h2>{title(current)}</h2><button type="button" disabled={!current.dirtySlots.length} onClick={() => void mutate(() => discardNeutralEdits(current.id))}>{t('neutral.revert')}</button></div>
        <p className="neutral-location">X {current.position[0]} / Y {current.position[1]} / Z {current.position[2]} · {current.position[2] === 0 ? t('neutral.surface') : t('neutral.underground')}
          {current.owner !== null && <> · {current.owner < 8 ? colorName(current.owner) : t('neutral.neutralOwner')}</>}</p>
        {current.army ? <ArmyEditor compact allowEmpty={current.kind !== 'wandering'} key={current.id} slots={current.army} choices={(data?.choices ?? []).filter(c => current.kind !== 'wandering' || current.allowedCreatures.includes(c.id) || c.id === current.creatureId)} readOnlyReason={current.readOnlyReason}
          dirtyFields={new Set(current.dirtySlots.map(slot => `Garrison slot ${slot + 1}`))} dirtyPrefix="Garrison slot"
          onApplyEdit={request => { if (request.army) void mutate(() => applyNeutralEdit(current.id, request.army!)); }} />
          : <div className="neutral-wandering-row"><CreaturePortrait id={current.creatureId} compact /><span>{current.name ?? `#${current.creatureId}`}</span><span>{t('neutral.quantityUnknown')}</span></div>}
        {current.readOnlyReason && <p className="fineprint">{t('ui.readOnly')} · {t('neutral.needsEvidence')}</p>}
        <details className="neutral-diagnostics"><summary>{t('neutral.diagnostics')}</summary>
          <p className="fineprint">{t('neutral.exists')}. {t('neutral.statusNote')}</p>
          {current.readOnlyReason && <p className="fineprint">{current.readOnlyReason}</p>}
        {current.initialArmy && <><h3>{t('neutral.initialMap')}</h3><p className="fineprint">{t('neutral.initialNote')}</p>
          <ul>{current.initialArmy.filter(s => s.creatureId !== null || s.count > 0).map((s, i) => <li key={i}>{s.name ?? (s.creatureId === null ? t('neutral.random') : `#${s.creatureId}`)}: {s.count}</li>)}</ul>
        </>}
        </details>
      </section>}
    </div>
  </>;
}

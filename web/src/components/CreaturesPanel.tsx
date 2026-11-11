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
  const [kind, setKind] = useState<'wandering' | 'garrison'>('wandering');
  const [level, setLevel] = useState('all');
  const [requests, setRequests] = useState(0);
  const generation = useRef(0);
  const queue = useRef(Promise.resolve());
  useEffect(() => {
    const token = ++generation.current;
    setData(null); setError(null);
    listNeutralArmies().then(result => {
      if (generation.current === token) setData(result);
    }, cause => { if (generation.current === token) setError(describeError(cause)); });
    return () => { generation.current++; };
  }, [epoch]);
  const mutate = (action: () => Promise<NeutralListDto>) => {
    const token = generation.current;
    setRequests(n => n + 1); onBusy(1); setError(null);
    queue.current = queue.current.then(async () => {
    try {
      const result = await action();
      if (generation.current === token) setData(result);
      await onChanged();
    } catch (cause) {
      if (generation.current === token) setError(describeError(cause));
    } finally {
      setRequests(n => n - 1); onBusy(-1);
    }
    });
  };
  const title = (g: NeutralArmyDto) => g.kind === 'wandering' ? g.name ?? `${t('ui.creature')} #${g.creatureId}` : `${t('neutral.garrisons')} #${g.id}`;
  const visible = data?.groups.filter(g => {
    const text = `${title(g)} ${g.id} ${g.position.join(', ')} ${g.army?.map(s => s.name ?? s.creatureId).join(' ') ?? ''}`.toLowerCase();
    return g.kind === kind && (level === 'all' || String(g.position[2]) === level)
      && query.toLowerCase().split(/\s+/).every(part => text.includes(part));
  }) ?? [];
  const current = visible.find(g => g.id === selected) ?? visible[0];
  const categoryCount = data?.groups.filter(g => g.kind === kind).length ?? 0;
  return <>
    <section className="hero-list neutral-list" aria-label={t('neutral.creatures')}>
      <div className="neutral-categories" aria-label={t('neutral.objectType')}>
        {(['wandering', 'garrison'] as const).map(category => <button type="button" key={category} aria-pressed={kind === category} onClick={() => setKind(category)}>
          {category === 'wandering' ? t('neutral.wandering') : t('neutral.garrisons')} <span>{data?.groups.filter(g => g.kind === category).length ?? 0}</span>
        </button>)}
      </div>
      <div className="hero-list-filters">
        <input type="search" aria-label={t('neutral.search')} placeholder={t('neutral.search')} value={query} onChange={e => setQuery(e.target.value)} />
        <select aria-label={t('neutral.level')} value={level} onChange={e => setLevel(e.target.value)}>
          <option value="all">{t('neutral.allLevels')}</option><option value="0">{t('neutral.surface')}</option><option value="1">{t('neutral.underground')}</option>
        </select>
      </div>
      {(query || level !== 'all') && <div className="neutral-filter-count">{visible.length} / {categoryCount}</div>}
      <ul className="hero-rows" aria-label={t('neutral.creatures')}>
        {visible.map(g => <li key={g.id}><button type="button" className={`hero-row${current?.id === g.id ? ' selected' : ''}`} aria-pressed={current?.id === g.id} onClick={() => setSelected(g.id)}>
          <CreaturePortrait id={g.creatureId ?? g.army?.find(s => !s.empty)?.creatureId ?? null} compact />
          <span className="neutral-row-text"><strong title={title(g)}>{title(g)}</strong><small title={g.army ? g.army.filter(s => !s.empty).map(s => `${s.count} ${s.name ?? `#${s.creatureId}`}`).join(', ') : t('neutral.quantityUnknown')}>
            {g.army ? g.army.filter(s => !s.empty).map(s => `${s.count} ${s.name ?? `#${s.creatureId}`}`).join(', ') || t('ui.empty') : t('neutral.quantityUnknown')}</small></span>
          <small className="neutral-row-position" title={g.position[2] === 0 ? t('neutral.surface') : t('neutral.underground')}>{g.position.join(' / ')}</small>
        </button></li>)}
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

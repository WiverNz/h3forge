import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { t } from '../i18n/index';
import { colorName } from '../i18n/labels';
import { playerColor } from '../lib/playerColor';
import { readListSort, writeListSort, toggleFacet, type ListSort } from '../lib/listView';

export function useListSort(list: string, keys: readonly string[], defaultKey = 'original') {
  const { i18n } = useTranslation();
  const [sort, setSort] = useState(() => readListSort(list, keys, defaultKey));
  function changeSort(next: ListSort) { setSort(next); writeListSort(list, next); }
  return { sort, changeSort, locale: i18n.resolvedLanguage ?? 'en', resetSort: () => changeSort({ key: defaultKey, direction: 'asc' }) };
}

export interface FacetOption { value: string; label: string; color?: string }
export function Facet({ label, options, selected, onChange, searchable = false }: {
  label: string; options: FacetOption[]; selected: string[]; onChange: (value: string[]) => void; searchable?: boolean;
}) {
  const [query, setQuery] = useState('');
  return <fieldset className="list-facet"><legend>{label}</legend>
    {searchable && <input type="search" aria-label={label} placeholder={label} value={query} onChange={event => setQuery(event.target.value)} />}
    <div className="list-facet-options">{options.filter(option => option.label.toLocaleLowerCase().includes(query.toLocaleLowerCase())).map(option =>
      <label key={option.value} data-player-color={option.color}>
        <input type="checkbox" checked={selected.includes(option.value)} onChange={() => onChange(toggleFacet(selected, option.value))} />
        {option.color && <span className="list-color-swatch" aria-hidden="true" />}{option.label}
      </label>)}</div>
  </fieldset>;
}
export function OwnerFacet({ selected, onChange }: { selected: string[]; onChange: (value: string[]) => void }) {
  return <Facet label={t('lists.owner')} selected={selected} onChange={onChange} options={[
    ...Array.from({ length: 8 }, (_, color) => ({ value: String(color), label: colorName(color), color: playerColor({ kind: 'player', player: color }) })),
    { value: 'neutral', label: t('lists.neutral'), color: 'neutral' }, { value: 'unknown', label: t('ui.unknown') },
  ]} />;
}

export function ListControls({ sort, onSort, options, active, onClear, onReset, hidden, children }: {
  sort: ListSort; onSort: (value: ListSort) => void; options: { value: string; label: string }[];
  active: number; onClear: () => void; onReset: () => void; hidden: boolean; children: ReactNode;
}) {
  const id = useId();
  const panel = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  // Native popovers escape the scrolling list's clipping and support light dismissal.
  useEffect(() => {
    const element = panel.current;
    if (!element) return;
    const toggle = (event: Event) => {
      const shown = (event as ToggleEvent).newState === 'open';
      setOpen(shown);
      if (shown) element.querySelector<HTMLElement>('input, select, button')?.focus();
      if (!shown && (document.activeElement === document.body || element.contains(document.activeElement))) button.current?.focus();
    };
    element.addEventListener('toggle', toggle);
    return () => element.removeEventListener('toggle', toggle);
  }, []);
  function toggle() {
    const element = panel.current;
    const rect = button.current?.getBoundingClientRect();
    if (!element || !rect) return;
    element.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - 304))}px`;
    element.style.top = `${Math.min(rect.bottom + 4, window.innerHeight - 180)}px`;
    element.style.maxHeight = `${Math.max(150, window.innerHeight - rect.bottom - 12)}px`;
    element.togglePopover();
  }
  return <>
    <div className="list-controls">
      <label className="list-sort-label">{t('lists.sort')}<select aria-label={t('lists.sort')} title={options.find(option => option.value === sort.key)?.label} value={sort.key} onChange={event => onSort({ ...sort, key: event.target.value })}>
        {options.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}
      </select></label>
      <button type="button" className="list-direction" aria-label={t(sort.direction === 'asc' ? 'lists.ascending' : 'lists.descending')}
        title={t(sort.direction === 'asc' ? 'lists.ascending' : 'lists.descending')} onClick={() => onSort({ ...sort, direction: sort.direction === 'asc' ? 'desc' : 'asc' })}>{sort.direction === 'asc' ? '↑' : '↓'}</button>
      <button type="button" ref={button} aria-expanded={open} aria-controls={id} onClick={toggle}>{t('lists.filter')}{active > 0 && ` (${active})`}</button>
      <div ref={panel} id={id} popover="auto" className="list-filter-popover" aria-label={t('lists.filter')}>
        {children}
        <div className="list-filter-actions"><button type="button" onClick={onClear}>{t('lists.clear')}</button><button type="button" onClick={onReset}>{t('lists.reset')}</button></div>
      </div>
    </div>
    {(hidden || active > 0) && <div className="list-filter-summary">{hidden && <span>{t('lists.hidden')}</span>}<button type="button" onClick={onClear}>{t('lists.clear')}</button></div>}
  </>;
}

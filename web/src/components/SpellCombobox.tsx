import { t } from '../i18n/index';
import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { computePopupPosition, type PopupPosition } from '../lib/creatureOptions';
import { SpellIcon } from './SpellIcon';

/**
 * Searchable spell picker with icons, sharing the creature selector's
 * placement and styling. With `value` it replaces that spell; without it the
 * trigger reads as an "add" control.
 */
export function SpellCombobox({ choices, value, valueName, placeholder, disabled = false, onChange }: {
  value?: number;
  valueName?: string;
  placeholder: string;
  choices: { id: number; name: string }[];
  disabled?: boolean;
  onChange: (id: number) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const listId = useId();
  const [pos, setPos] = useState<PopupPosition | null>(null);
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const filtered = choices.filter((choice) =>
    choice.name.toLowerCase().includes(query.trim().toLowerCase()));
  const current = filtered[active];
  const close = (focus = false) => { setPos(null); if (focus) trigger.current?.focus(); };
  const choose = (id: number) => { onChange(id); close(true); };
  const open = () => {
    if (!trigger.current || choices.length === 0) return;
    setQuery(''); setActive(Math.max(0, choices.findIndex((choice) => choice.id === value)));
    setPos(computePopupPosition(trigger.current.getBoundingClientRect(),
      { width: window.innerWidth, height: window.innerHeight }, choices.length));
  };
  useLayoutEffect(() => {
    if (!pos) return;
    input.current?.focus();
    list.current?.children[active]?.scrollIntoView({ block: 'nearest' });
  }, [pos, active, query]);
  useEffect(() => {
    if (!pos) return;
    const outside = (event: Event) => {
      if (event.target instanceof Node && root.current?.contains(event.target)) return;
      setPos(null);
    };
    const resize = () => setPos(null);
    document.addEventListener('pointerdown', outside);
    document.addEventListener('scroll', outside, true);
    window.addEventListener('resize', resize);
    return () => {
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('scroll', outside, true);
      window.removeEventListener('resize', resize);
    };
  }, [pos]);
  const label = value !== undefined ? valueName ?? t('ui.spellValue2', { value1: value }) : placeholder;
  return <div className="creature-combobox spell-combobox" ref={root}>
    <button type="button" ref={trigger} className="creature-combobox-trigger"
      aria-label={value === undefined ? placeholder : t('ui.replaceValue', { value1: label })}
      disabled={disabled || choices.length === 0} aria-haspopup="listbox" aria-expanded={pos !== null}
      aria-controls={pos ? listId : undefined} onClick={() => pos ? close() : open()}
      onKeyDown={(event) => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); open(); }
      }}>{value !== undefined && <SpellIcon id={value} />}
      <span className="creature-combobox-trigger-name">{label}</span>
      {!disabled && <span aria-hidden="true">▾</span>}</button>
    {pos && <div className="creature-combobox-popup" style={{ top: pos.top, left: pos.left, width: Math.max(pos.width, 220) }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') { event.preventDefault(); close(true); }
        else if (event.key === 'Tab') close();
        else if (event.key === 'Enter') { event.preventDefault(); if (current) choose(current.id); }
        else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault();
          if (filtered.length) setActive((index) =>
            (index + (event.key === 'ArrowDown' ? 1 : -1) + filtered.length) % filtered.length);
        }
      }}>
      <input ref={input} className="creature-combobox-search" role="combobox" aria-expanded="true"
        aria-controls={listId} aria-autocomplete="list" aria-label={t('ui.searchSpells')} placeholder={t('ui.searchSpells2')}
        aria-activedescendant={current ? `${listId}-${current.id}` : undefined}
        value={query} onChange={(event) => { setQuery(event.target.value); setActive(0); }} />
      <ul ref={list} id={listId} className="creature-combobox-list" role="listbox" aria-label={t('ui.spells')}
        style={{ maxHeight: pos.listHeight }}>
        {filtered.map((choice, index) => <li key={choice.id} id={`${listId}-${choice.id}`}
          className="creature-combobox-option skill-combobox-option" role="option" aria-selected={index === active}
          data-active={index === active} onMouseMove={() => setActive(index)} onClick={() => choose(choice.id)}>
          <SpellIcon id={choice.id} /><span>{choice.name}</span>
        </li>)}
      </ul>
      {filtered.length === 0 && <p className="creature-combobox-none">{t('ui.noSpellsMatch')}</p>}
    </div>}
  </div>;
}

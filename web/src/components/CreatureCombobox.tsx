import { t, i18n } from '../i18n/index';
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { CreaturePortrait } from './CreaturePortrait';
import type { CreatureDto } from '../types/api';
import {
  buildCreatureOptions,
  computePopupPosition,
  filterCreatureOptions,
  CREATURE_OPTION_HEIGHT,
  type CreatureOption,
  type PopupPosition,
} from '../lib/creatureOptions';

const OVERSCAN = 4;

/**
 * Compact image dropdown for picking a creature in an army slot. Replaces the
 * native <select>: the closed trigger shows the selected portrait and name,
 * the popup holds a name filter plus a virtualized listbox. Options mount
 * only while visible, so portraits are requested lazily through the shared
 * cache instead of fetching the whole catalog on every open. The popup is
 * position:fixed and flips upward near the viewport edge, so it is never
 * clipped by the scrollable Army panel; any scroll outside the popup closes
 * it instead.
 */
export function CreatureCombobox({ value, choices, allowEmpty = true, onChange }: {
  /** Current creature id as a string; '' means an empty stack. */
  value: string;
  choices: CreatureDto[];
  allowEmpty?: boolean;
  onChange: (value: string) => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const listId = useId();
  const options = useMemo(() => buildCreatureOptions(choices).filter(option => allowEmpty || option.id !== null).map(option => option.id === null ? { ...option, name: t('ui.empty') } : option), [choices, allowEmpty, i18n.resolvedLanguage]);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [pos, setPos] = useState<PopupPosition | null>(null);

  const filtered = useMemo(() => filterCreatureOptions(options, query), [options, query]);
  const selected = value === '' ? null : Number(value);
  const selectedName = selected === null
    ? t('ui.empty')
    : options.find((option) => option.id === selected)?.name ?? t('ui.creatureValue', { value1: selected });
  const activeOption: CreatureOption | undefined = filtered[active];

  const closePopup = (refocus: boolean) => {
    setOpen(false);
    setPos(null);
    if (refocus) triggerRef.current?.focus();
  };

  const openPopup = () => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    setPos(computePopupPosition(
      trigger.getBoundingClientRect(),
      { width: window.innerWidth, height: window.innerHeight },
      options.length,
    ));
    setQuery('');
    setScrollTop(0);
    const selectedIndex = options.findIndex((option) => option.id === selected);
    setActive(selectedIndex < 0 ? 0 : selectedIndex);
    setOpen(true);
  };

  const choose = (option: CreatureOption) => {
    onChange(option.id === null ? '' : String(option.id));
    closePopup(true);
  };

  // Focus the filter on open and keep the active option scrolled into view.
  useLayoutEffect(() => {
    if (!open) return;
    inputRef.current?.focus();
    const list = listRef.current;
    if (!list || !pos) return;
    const itemTop = active * CREATURE_OPTION_HEIGHT;
    if (itemTop < list.scrollTop) {
      list.scrollTop = itemTop;
    } else if (itemTop + CREATURE_OPTION_HEIGHT > list.scrollTop + pos.listHeight) {
      list.scrollTop = itemTop + CREATURE_OPTION_HEIGHT - pos.listHeight;
    }
    setScrollTop(list.scrollTop);
  }, [open, active, pos]);

  // Close on outside interaction; ancestor scrolling would detach the fixed
  // popup from its trigger, so close instead of repositioning.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: Event) => {
      if (rootRef.current && event.target instanceof Node && !rootRef.current.contains(event.target)) {
        closePopup(false);
      }
    };
    const onScroll = (event: Event) => {
      if (rootRef.current && event.target instanceof Node && rootRef.current.contains(event.target)) return;
      closePopup(false);
    };
    const onResize = () => closePopup(false);
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', onResize);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onResize);
    };
  }, [open]);

  const onPopupKeyDown = (event: KeyboardEvent) => {
    const length = filtered.length;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (length > 0) setActive((index) => (index + 1) % length);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (length > 0) setActive((index) => (index - 1 + length) % length);
    } else if (event.key === 'Home') {
      event.preventDefault();
      setActive(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      if (length > 0) setActive(length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (activeOption) choose(activeOption);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      closePopup(true);
    } else if (event.key === 'Tab') {
      closePopup(false);
    }
  };

  // Clamp against the filtered list: a stale deep scroll offset would slice
  // past short result sets and flash an empty list.
  const listHeight = pos ? Math.min(pos.listHeight, filtered.length * CREATURE_OPTION_HEIGHT) : 0;
  const maxScroll = Math.max(0, filtered.length * CREATURE_OPTION_HEIGHT - listHeight);
  const top = Math.min(scrollTop, maxScroll);
  const start = Math.max(0, Math.floor(top / CREATURE_OPTION_HEIGHT) - OVERSCAN);
  const end = Math.min(filtered.length, Math.ceil((top + listHeight) / CREATURE_OPTION_HEIGHT) + OVERSCAN);
  const visibleOptions = filtered.slice(start, end);
  const optionId = (option: CreatureOption) =>
    `${listId}-option-${option.id === null ? 'empty' : option.id}`;

  return <div className="creature-combobox" ref={rootRef}>
    <button type="button" ref={triggerRef} className="creature-combobox-trigger"
      aria-haspopup="listbox" aria-expanded={open} aria-controls={listId}
      onClick={() => (open ? closePopup(false) : openPopup())}
      onKeyDown={(event) => {
        if (open && event.key === 'Escape') {
          event.preventDefault();
          closePopup(true);
        } else if (!open && (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ')) {
          event.preventDefault();
          openPopup();
        } else if (open && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
          event.preventDefault();
          inputRef.current?.focus();
        }
      }}>
      {selected === null
        ? <span className="creature-combobox-empty-icon" aria-hidden="true">-</span>
        : <CreaturePortrait id={selected} option />}
      <span className="creature-combobox-trigger-name">{selectedName}</span>
      <span className="creature-combobox-caret" aria-hidden="true">{open ? '▲' : '▼'}</span>
    </button>
    {open && pos && <div className="creature-combobox-popup" style={{ top: pos.top, left: pos.left, width: pos.width }}
      onKeyDown={onPopupKeyDown}>
      <input ref={inputRef} className="creature-combobox-search" role="combobox"
        aria-expanded="true" aria-controls={listId} aria-autocomplete="list"
        aria-activedescendant={activeOption ? optionId(activeOption) : undefined}
        aria-label={t('ui.searchCreatures')} placeholder={t('ui.searchCreatures2')}
        value={query} onChange={(event) => { setQuery(event.target.value); setActive(0); setScrollTop(0); }} />
      <ul ref={listRef} id={listId} className="creature-combobox-list" role="listbox"
        aria-label={t('ui.creatures')} style={{ height: listHeight }}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}>
        <li aria-hidden="true" role="presentation" className="creature-combobox-spacer" style={{ height: start * CREATURE_OPTION_HEIGHT }} />
        {visibleOptions.map((option, offset) => {
          const index = start + offset;
          return <li key={option.id === null ? 'empty' : option.id} id={optionId(option)}
            role="option" aria-selected={index === active} data-active={index === active}
            className="creature-combobox-option"
            style={{ top: index * CREATURE_OPTION_HEIGHT, height: CREATURE_OPTION_HEIGHT }}
            onMouseMove={() => setActive(index)}
            onClick={() => choose(option)}>
            {option.id === null
              ? <span className="creature-combobox-empty-icon" aria-hidden="true">-</span>
              : <CreaturePortrait id={option.id} option />}
            <span className="creature-combobox-option-name">{option.name}</span>
          </li>;
        })}
        <li aria-hidden="true" role="presentation" className="creature-combobox-spacer"
          style={{ height: Math.max(0, (filtered.length - end) * CREATURE_OPTION_HEIGHT) }} />
      </ul>
      {filtered.length === 0 && <p className="creature-combobox-none">{t('ui.noCreaturesMatch')}</p>}
    </div>}
  </div>;
}

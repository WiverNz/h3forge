import { t } from '../i18n/index';
import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { ArtifactIcon } from './ArtifactIcon';

export interface ArtifactOption { key: string; icon: number; name: string; detail?: string }

/** Shared searchable selector. Artwork is requested only as rows enter view. */
export function ArtifactPicker({ title, options, onChoose, onClose, children }: {
  title: string; options: ArtifactOption[]; onChoose: (key: string) => void;
  onClose: () => void; children?: ReactNode;
}) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const id = useId();
  const list = useRef<HTMLUListElement>(null);
  const filtered = options.filter(item => `${item.name} ${item.detail ?? ''}`.toLowerCase().includes(query.trim().toLowerCase()));
  useEffect(() => {
    const trigger = document.activeElement;
    return () => { if (trigger instanceof HTMLElement && trigger.isConnected) trigger.focus(); };
  }, []);
  useEffect(() => { list.current?.children[active]?.scrollIntoView({ block: 'nearest' }); }, [active]);
  return <div className="artifact-picker" role="region" aria-label={title}
    onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); onClose(); } }}>
    <div className="skills-head"><strong>{title}</strong><button type="button" onClick={onClose} aria-label={t('ui.closeArtifactPicker')}>×</button></div>
    {children}
    <input autoFocus type="search" role="combobox" aria-label={t('ui.searchArtifactsAndSpells')}
      aria-expanded="true" aria-controls={id} aria-autocomplete="list"
      aria-activedescendant={filtered[active] ? `${id}-${active}` : undefined}
      placeholder={t('ui.searchArtifactsAndSpells2')} value={query}
      onChange={event => { setQuery(event.target.value); setActive(0); list.current?.scrollTo(0, 0); }}
      onKeyDown={event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault();
          if (filtered.length) setActive(index => (index + (event.key === 'ArrowDown' ? 1 : -1) + filtered.length) % filtered.length);
        } else if (event.key === 'Enter' && filtered[active]) { event.preventDefault(); onChoose(filtered[active].key); }
      }} />
    <ul id={id} ref={list} role="listbox" aria-label={title} className="artifact-options">
      {filtered.map((item, index) => <li key={item.key} id={`${id}-${index}`} role="option"
        aria-selected={index === active} className="artifact-option" onClick={() => onChoose(item.key)}>
        <ArtifactIcon id={item.icon} /><span>{item.name}{item.detail && <small>{item.detail}</small>}</span>
      </li>)}
    </ul>
    {!filtered.length && <p className="fineprint">{t('ui.noCompatibleItemsFound')}</p>}
    <p className="fineprint">{t('ui.toBrowseEnterToSelectEscapeToClose')}</p>
  </div>;
}

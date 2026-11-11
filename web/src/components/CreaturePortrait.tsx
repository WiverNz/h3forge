import { t } from '../i18n/index';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { getCreaturePortrait, getCreaturePortraitVersion, subscribeCreaturePortraits } from '../api/resources';
import type { PortraitDto } from '../types/api';
import { Portrait } from './Portrait';

/** Shared by the Army editor, compact list previews, and the creature combobox. */
export function CreaturePortrait({ id, compact = false, option = false }: {
  id: number | null;
  compact?: boolean;
  option?: boolean;
}) {
  const version = useSyncExternalStore(subscribeCreaturePortraits, getCreaturePortraitVersion, getCreaturePortraitVersion);
  const [loaded, setLoaded] = useState<{ id: number; version: number; portrait: PortraitDto } | null>(null);
  useEffect(() => {
    let active = true;
    if (id !== null) getCreaturePortrait(id).then(
      (portrait) => { if (active) setLoaded({ id, version, portrait }); },
      () => { if (active) setLoaded(null); },
    );
    return () => { active = false; };
  }, [id, version]);
  const portrait = loaded?.id === id && loaded?.version === version ? loaded.portrait : null;
  const className = compact ? 'hero-army-icon' : option ? 'creature-option-icon' : 'creature-portrait';
  return <span className={className}
    aria-label={id === null ? t('ui.emptyStack') : t('ui.creatureValue', { value1: id })}>
    <Portrait portrait={portrait} size="small" alt={t('ui.creaturePortrait')} />
  </span>;
}

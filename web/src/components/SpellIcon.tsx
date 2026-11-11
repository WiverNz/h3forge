import { t } from '../i18n/index';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { getSpellIcon, getSpellIconVersion, subscribeSpellIcons } from '../api/resources';
import type { PortraitDto } from '../types/api';

/**
 * Compact spellbook icon for one game spell ID (0-69). Resolved icons render
 * the decoded 78x65 spells.def frame (palette index 0 already transparent in
 * the decoded PNG); unavailable or unverified IDs render a small placeholder
 * with the reason, never a neighbouring frame.
 */
export function SpellIcon({ id }: { id: number }) {
  const version = useSyncExternalStore(subscribeSpellIcons, getSpellIconVersion, getSpellIconVersion);
  const [loaded, setLoaded] = useState<{ id: number; version: number; icon: PortraitDto } | null>(null);
  useEffect(() => {
    let active = true;
    getSpellIcon(id).then(
      (icon) => { if (active) setLoaded({ id, version, icon }); },
      () => { if (active) setLoaded(null); },
    );
    return () => { active = false; };
  }, [id, version]);
  const icon = loaded?.id === id && loaded?.version === version ? loaded.icon : null;
  if (!icon || !icon.resolved || !icon.pngBase64) {
    const reason = icon?.reason ?? t('ui.iconNotLoaded');
    return <span className="spell-icon spell-icon-placeholder" role="img"
      aria-label={t('ui.spellIconUnavailableValue', { value1: reason })} title={reason}>?</span>;
  }
  return <img className="spell-icon" src={`data:image/png;base64,${icon.pngBase64}`}
    alt={t('ui.spellIcon')} width={33} height={27} />;
}

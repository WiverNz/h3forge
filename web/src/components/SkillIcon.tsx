import { t } from '../i18n/index';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { getSkillIcon, getSkillIconVersion, subscribeSkillIcons } from '../api/resources';
import type { PortraitDto } from '../types/api';

/** Shared cached mastery-specific skill artwork; unknown or unavailable resources stay placeholders. */
export function SkillIcon({ id, mastery }: { id: number; mastery: number }) {
  const [failed, setFailed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const version = useSyncExternalStore(subscribeSkillIcons, getSkillIconVersion, getSkillIconVersion);
  const [loaded, setLoaded] = useState<{ id: number; mastery: number; version: number; icon: PortraitDto } | null>(null);
  useEffect(() => {
    let active = true;
    setFailed(false);
    setError(null);
    getSkillIcon(id, mastery).then(
      (icon) => { if (active) setLoaded({ id, mastery, version, icon }); },
      (reason: unknown) => {
        if (active) {
          setLoaded(null);
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      },
    );
    return () => { active = false; };
  }, [id, mastery, version]);
  const icon = loaded?.id === id && loaded?.mastery === mastery && loaded?.version === version ? loaded.icon : null;
  if (failed || !icon || !icon.resolved || !icon.pngBase64) {
    const reason = failed ? t('ui.skillImageCouldNotBeDisplayed') : error ?? icon?.reason ?? t('ui.iconNotLoaded');
    return <span className="skill-icon skill-icon-placeholder" role="img"
      aria-label={t('ui.skillIconUnavailableValue', { value1: reason })} title={reason}>?</span>;
  }
  return <img className="skill-icon" src={`data:image/png;base64,${icon.pngBase64}`}
    onError={() => setFailed(true)} alt="" width={24} height={24} />;
}

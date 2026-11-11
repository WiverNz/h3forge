import { t } from '../i18n/index';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { getTownIcon, getTownIconVersion, subscribeTownIcons } from '../api/resources';
import type { PortraitDto } from '../types/api';

/**
 * Original town icon for one town type (H3F-022): the 48x32 list picture or
 * the 58x64 header picture, in its fort or village variant. Unresolved types
 * render a neutral placeholder with the reason; no other artwork is used.
 */
export function TownIcon({
  townType,
  fort,
  large = false,
  label,
}: {
  townType: number;
  fort: boolean;
  large?: boolean;
  label: string;
}) {
  const version = useSyncExternalStore(subscribeTownIcons, getTownIconVersion, getTownIconVersion);
  const key = `${townType}:${fort}:${large}:${version}`;
  const [loaded, setLoaded] = useState<{ key: string; icon: PortraitDto } | null>(null);
  useEffect(() => {
    let active = true;
    getTownIcon(townType, fort, large).then(
      (icon) => { if (active) setLoaded({ key, icon }); },
      () => { if (active) setLoaded(null); },
    );
    return () => { active = false; };
  }, [townType, fort, large, key]);
  const icon = loaded?.key === key ? loaded.icon : null;
  const className = large ? 'town-icon town-icon-large' : 'town-icon town-icon-small';
  if (!icon || !icon.resolved || !icon.pngBase64) {
    const reason = icon?.reason ?? t('ui.iconNotLoaded');
    return (
      <span className={`${className} town-icon-placeholder`} role="img"
        aria-label={t('ui.valueIconUnavailableValue', { value1: label, value2: reason })} title={reason}>?</span>
    );
  }
  return (
    <img className={className} src={`data:image/png;base64,${icon.pngBase64}`}
      alt={t('ui.valueTownIcon', { value1: label })} width={large ? 58 : 48} height={large ? 64 : 32} />
  );
}

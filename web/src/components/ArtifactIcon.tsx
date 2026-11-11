import { t } from '../i18n/index';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { getArtifactIcon, getArtifactIconVersion, subscribeArtifactIcons } from '../api/resources';
import type { PortraitDto } from '../types/api';
import { artifactIconError } from '../lib/artifactIconError';

/**
 * 44x44 artifact icon from `Artifact.def` (frame mapped by the embedded
 * `HrArt###.pcx` name). Unresolved IDs (unknown artifacts, missing
 * artwork) render a small placeholder, never a neighbouring icon.
 */
export function ArtifactIcon({ id, size = 44 }: { id: number; size?: number }) {
  const element = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    if (typeof IntersectionObserver === 'undefined') { setVisible(true); return; }
    const observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) { setVisible(true); observer.disconnect(); }
    });
    if (element.current) observer.observe(element.current);
    return () => observer.disconnect();
  }, []);
  const version = useSyncExternalStore(subscribeArtifactIcons, getArtifactIconVersion, getArtifactIconVersion);
  const [loaded, setLoaded] = useState<{ id: number; version: number; icon: PortraitDto | null; error?: { kind: 'load' | 'decode'; details: string } } | null>(null);
  useEffect(() => {
    if (!visible || id < 0) return;
    let active = true;
    getArtifactIcon(id).then(
      (icon) => { if (active) setLoaded({ id, version, icon }); },
      (error: unknown) => {
        if (active) setLoaded({ id, version, icon: null, error: { kind: 'load', details: artifactIconError(error) } });
      },
    );
    return () => { active = false; };
  }, [id, version, visible]);
  const current = loaded?.id === id && loaded?.version === version ? loaded : null;
  const icon = current?.icon;
  if (current?.error || !icon || !icon.resolved || !icon.pngBase64) {
    const error = current?.error?.kind === 'decode' ? t('ui.artifactValuePngWasReturnedButTheWebviewCould', { value1: id })
      : current?.error ? t('ui.getArtifactIconValueFailedValue', { value1: id, value2: current.error.details }) : null;
    const reason = id < 0 ? t('ui.noVerifiedArtifactIdForThisEntry') : error ?? icon?.reason
      ?? (icon?.resolved ? t('ui.getArtifactIconValueReturnedNoPngData', { value1: id })
        : visible ? t('ui.loadingArtifactValue', { value1: id }) : t('ui.artifactValueImageIsWaitingToEnterView', { value1: id }));
    return <span ref={element} style={{ width: size, height: size }} className="artifact-icon artifact-icon-placeholder" role="img" aria-label={t('ui.artifactIconUnavailable')}
      aria-description={reason} title={reason}>◇</span>;
  }
  return <img className="artifact-icon" src={`data:image/png;base64,${icon.pngBase64}`} alt={t('ui.artifactIcon')}
    data-artifact-id={id}
    onError={() => setLoaded({ id, version, icon, error: { kind: 'decode', details: '' } })}
    width={size} height={size} />;
}


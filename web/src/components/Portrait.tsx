import { t } from '../i18n/index';
import type { PortraitDto } from '../types/api';

/**
 * One hero portrait. Resolved portraits render the decoded game image;
 * unresolved ones render a clearly generic placeholder with the reason, so a
 * missing or custom portrait never shows a wrong face.
 */
export function Portrait({
  portrait,
  size,
  alt = t('ui.heroPortrait'),
}: {
  portrait: PortraitDto | null | undefined;
  size: 'large' | 'small';
  alt?: string;
}) {
  if (!portrait || !portrait.resolved || !portrait.pngBase64) {
    const reason = portrait?.reason ?? t('ui.portraitNotLoaded');
    return (
      <span
        className={`portrait portrait-placeholder portrait-${size}`}
        role="img"
        aria-label={t('ui.portraitUnavailableValue', { value1: reason })}
        title={reason}
      >
        ?
      </span>
    );
  }
  return (
    <img
      className={`portrait portrait-${size}`}
      src={`data:image/png;base64,${portrait.pngBase64}`}
      alt={alt}
      width={size === 'large' ? 58 : 32}
      height={size === 'large' ? 64 : 32}
    />
  );
}

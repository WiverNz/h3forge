import { reasonText, type Reason } from '../i18n/reasons';
import { t } from '../i18n/index';
import type { EquipmentItemDto } from '../types/api';
import { ArtifactIcon } from './ArtifactIcon';

export function artifactLabel(item: EquipmentItemDto) {
  return item.kind === 'scroll' ? t('ui.spellScrollValue', { value1: item.spellName ?? `Spell ${item.spellId}` })
    : item.kind === 'empty' ? t('ui.empty') : item.name ?? t('ui.unknownArtifactValue', { value1: item.artifactId !== null ? ` (ID ${item.artifactId})` : '' });
}

export function ArtifactSlot({ item, label, reason, effect, reservedBy, dirty, selected, onClick }: {
  item: EquipmentItemDto; label: string; reason?: Reason | null; effect?: string | null;
  /** Worn combination artifact that takes this empty slot. */
  reservedBy?: string | null;
  dirty?: boolean; selected?: boolean; onClick: () => void;
}) {
  const description = [reservedBy ? t('ui.takenByValue', { value1: reservedBy }) : artifactLabel(item), label, effect, reservedBy ? null : reasonText(reason)]
    .filter(Boolean).join(' · ');
  return <button type="button" className="artifact-slot" aria-label={description}
    aria-disabled={!!reason} aria-pressed={selected} title={description}
    data-dirty={dirty} data-readonly={!!reason} data-reserved={!!reservedBy} onClick={onClick}>
    {item.kind !== 'empty' && <ArtifactIcon id={item.kind === 'unknown' ? -1 : item.kind === 'scroll' ? 1 : item.artifactId ?? -1} />}
    {reason && <span className="artifact-lock" aria-hidden="true">◆</span>}
    {dirty && <span className="dirty-dot" />}
  </button>;
}

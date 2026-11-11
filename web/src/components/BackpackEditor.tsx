import { ReasonText, type Reason } from '../i18n/reasons';
import { t } from '../i18n/index';
import { useState } from 'react';
import type { BackpackItemDto, HeroEditRequest, HeroEditResponseDto, SpellDto } from '../types/api';
import { ArtifactPicker } from './ArtifactPicker';
import { ArtifactSlot, artifactLabel } from './ArtifactSlot';

export function BackpackEditor({ slots, choices, spells, readOnlyReason, dirtyFields, onApplyEdit, target, setTarget }: {
  slots: BackpackItemDto[]; choices: HeroEditResponseDto['artifactChoices']; spells: SpellDto[];
  readOnlyReason: Reason | null; dirtyFields: Set<string>; onApplyEdit: (request: HeroEditRequest) => void;
  target: number | null; setTarget: (slot: number | null) => void;
}) {
  const [moving, setMoving] = useState<number | null>(null);
  const selected = slots.find(item => item.slot === target);
  const options = [...choices.map(item => ({ key: `artifact:${item.id}`, icon: item.id, name: item.name,
    detail: [item.combination ? t('ui.combinationArtifact') : null, item.equipKind ?? t('ui.inventory'), item.effect].filter(Boolean).join(' · ') })),
    ...spells.map(spell => ({ key: `scroll:${spell.id}`, icon: 1, name: t('ui.spellScrollValue', { value1: spell.name }), detail: t('ui.scrollBackpackOnly') }))];
  const effects = new Map(choices.map(item => [item.id, item.effect]));
  const readonly = selected && (readOnlyReason ?? (!selected.editable ? t('ui.unsupportedItemPreservedInItsOriginalSlotReplacementRemoval') : null));
  return <section className="backpack-editor">
    <div className="skills-head"><h4>{t('ui.backpack')}</h4><span>{slots.filter(item => item.kind !== 'empty').length}/64</span></div>
    {readOnlyReason && <p className="fineprint">{<ReasonText reason={readOnlyReason} />}</p>}
    {moving !== null && <p className="fineprint">{t('ui.movingSlot')}{moving + 1}{t('ui.selectAnEmptySquare')}<button type="button" onClick={() => setMoving(null)}>{t('ui.cancelMove')}</button></p>}
    <div className="backpack-grid" aria-label={t('ui.64FixedBackpackPositions')}>
      {slots.map(item => <ArtifactSlot key={item.slot} item={item} label={`Backpack slot ${item.slot + 1}`}
        effect={effects.get(item.artifactId ?? -1)}
        reason={readOnlyReason ?? (!item.editable ? t('ui.unsupportedItemReadOnly') : null)}
        dirty={dirtyFields.has(`Backpack slot ${item.slot + 1}`)} selected={target === item.slot || moving === item.slot}
        onClick={() => {
          if (moving !== null) {
            const source = slots.find(slot => slot.slot === moving);
            if (readOnlyReason || item.kind !== 'empty' || !item.editable || !source?.editable) return;
            const value = source.kind === 'scroll' ? source.spellId : source.artifactId;
            if (value === null || (source.kind !== 'scroll' && source.kind !== 'artifact')) return;
            onApplyEdit({ backpackSlot: { slot: source.slot, item: { kind: 'empty' } },
              backpackDestination: { slot: item.slot, item: { kind: source.kind, value } } });
            setMoving(null); setTarget(null);
          } else setTarget(item.slot);
        }} />)}
    </div>
    {selected && <div className="artifact-selection">
      <p><strong>{t('ui.slotValueValue', { value1: selected.slot + 1, value2: artifactLabel(selected) })}</strong></p>
      {readonly ? <><p className="fineprint">{<ReasonText reason={readonly} />}</p><button type="button" onClick={() => setTarget(null)}>{t('ui.close')}</button></> :
        <ArtifactPicker key={target} title={t('ui.valueArtifactSlotValue', { value1: selected.kind === 'empty' ? t('ui.add') : t('ui.replace'), value2: selected.slot + 1 })}
          options={options} onClose={() => setTarget(null)} onChoose={key => {
            const [kind, value] = key.split(':');
            onApplyEdit({ backpackSlot: { slot: selected.slot, item: { kind: kind as 'artifact' | 'scroll', value: Number(value) } } });
            setTarget(null);
          }}>
          {selected.kind !== 'empty' && <div className="artifact-actions">
            <button type="button" onClick={() => { onApplyEdit({ backpackSlot: { slot: selected.slot, item: { kind: 'empty' } } }); setTarget(null); }}>{t('ui.remove')}</button>
            <button type="button" disabled={!slots.some(item => item.kind === 'empty' && item.editable)} onClick={() => { setMoving(selected.slot); setTarget(null); }}>{t('ui.moveTo')}</button>
          </div>}
        </ArtifactPicker>}
    </div>}
    <p className="fineprint">{t('ui.all64PositionsStayFixedHoverOrSelectFor')}</p>
  </section>;
}

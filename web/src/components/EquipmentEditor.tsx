import { ReasonText, type Reason } from '../i18n/reasons';
import { t } from '../i18n/index';
import type { BackpackItemDto, EquipmentSlotDto, HeroEditRequest, HeroEditResponseDto } from '../types/api';
import { ArtifactPicker } from './ArtifactPicker';
import { ArtifactSlot, artifactLabel } from './ArtifactSlot';

export function EquipmentEditor({ slots, backpack, choices, readOnlyReason, dirtyFields, onApplyEdit, target, setTarget }: {
  slots: EquipmentSlotDto[]; backpack: BackpackItemDto[];
  choices: HeroEditResponseDto['artifactChoices']; readOnlyReason: Reason | null;
  dirtyFields: Set<string>; onApplyEdit: (request: HeroEditRequest) => void;
  target: number | null; setTarget: (slot: number | null) => void;
}) {
  const selected = slots.find(slot => slot.slot === target);
  const byId = new Map(choices.map(item => [item.id, item]));
  const empty = backpack.find(item => item.kind === 'empty');
  const reason = selected ? readOnlyReason ?? selected.readOnlyReason : null;
  const wornId = selected?.item.kind === 'artifact' ? selected.item.artifactId : null;
  // Compatible backpack items first (equipping swaps with that backpack
  // slot), then every artifact that fits this slot (replaces in place).
  const fromBackpack = selected ? backpack.filter(item => item.kind === 'artifact' && item.editable
    && byId.get(item.artifactId ?? -1)?.equipKind === selected.kind) : [];
  const catalog = selected ? choices.filter(item => item.equipKind === selected.kind && item.id !== wornId) : [];
  // Combination artifacts that take this slot as one of their extra slots:
  // choosing one equips it in its own primary slot and clears this one.
  const viaSets = selected ? choices.filter(item => item.combination && item.equipKind !== selected.kind
    && item.alsoTakes.includes(selected.kind)) : [];
  const primarySlotFor = (kind: string | null) => {
    const candidates = slots.filter(slot => slot.kind === kind);
    return candidates.find(slot => slot.item.kind === 'empty' && !slot.reservedBy) ?? candidates[0];
  };
  const options = [
    ...fromBackpack.map(item => ({
      key: `backpack:${item.slot}`, icon: item.artifactId!, name: item.name ?? t('ui.artifact'),
      detail: [t('ui.fromBackpackSlotValueSwaps', { value1: item.slot + 1 }), byId.get(item.artifactId!)?.effect].filter(Boolean).join(' · '),
    })),
    ...catalog.map(item => ({
      key: `artifact:${item.id}`, icon: item.id, name: item.name,
      detail: [item.combination ? t('ui.combinationArtifact') : null, item.effect].filter(Boolean).join(' · '),
    })),
    ...viaSets.map(item => ({
      key: `set:${item.id}`, icon: item.id, name: item.name,
      detail: [t('ui.combinationArtifactWornInTheValueSlotTakesThis', { value1: primarySlotFor(item.equipKind)?.name ?? item.equipKind }),
        item.effect].filter(Boolean).join(' · '),
    })),
  ];
  const choose = (key: string) => {
    if (!selected || reason) return;
    const [kind, value] = key.split(':');
    if (kind === 'backpack') {
      const source = fromBackpack.find(item => item.slot === Number(value));
      if (!source || source.artifactId === null) return;
      onApplyEdit({ equipmentSlot: { slot: selected.slot, item: { kind: 'artifact', value: source.artifactId } },
        backpackSlot: { slot: source.slot, item: wornId !== null ? { kind: 'artifact', value: wornId } : { kind: 'empty' } } });
    } else if (kind === 'set') {
      const set = viaSets.find(item => item.id === Number(value));
      const primary = set && primarySlotFor(set.equipKind);
      if (!set || !primary) return;
      onApplyEdit({ equipmentSlot: { slot: primary.slot, item: { kind: 'artifact', value: set.id } },
        ...(selected.item.kind !== 'empty' ? { equipmentSecond: { slot: selected.slot, item: { kind: 'empty' as const } } } : {}) });
    } else {
      onApplyEdit({ equipmentSlot: { slot: selected.slot, item: { kind: 'artifact', value: Number(value) } } });
    }
    setTarget(null);
  };
  const renderSlot = (index: number, label: string) => {
    const slot = slots.find(item => item.slot === index);
    return slot && <div key={index} className={`equipment-position equipment-position-${index}`}>
      <small>{label}</small><ArtifactSlot item={slot.item} label={slot.name}
        reason={readOnlyReason ?? slot.readOnlyReason} effect={byId.get(slot.item.artifactId ?? -1)?.effect}
        reservedBy={slot.reservedBy}
        dirty={dirtyFields.has(`Equipment (${slot.name})`)} selected={target === index} onClick={() => setTarget(index)} />
    </div>;
  };
  return <section className="backpack-editor equipment-editor">
    <div className="skills-head"><h4>{t('ui.equipment')}</h4><span>{slots.filter(slot => slot.kind !== 'machine' && slot.kind !== 'spellbook' && slot.item.kind !== 'empty').length}{t('ui.14Equipped')}</span></div>
    {readOnlyReason && <p className="fineprint">{<ReasonText reason={readOnlyReason} />}</p>}
    <div className="equipment-body">
      {renderSlot(0, t('ui.helmet'))}{renderSlot(1, t('ui.cloak'))}{renderSlot(2, t('ui.necklace'))}
      {renderSlot(3, t('ui.weapon'))}{renderSlot(5, t('ui.armor'))}{renderSlot(4, t('ui.shield'))}
      {renderSlot(6, t('ui.leftRing'))}{renderSlot(8, t('ui.boots'))}{renderSlot(7, t('ui.rightRing'))}
    </div>
    <div className="equipment-accessories" aria-label={t('ui.miscellaneousSlots')}>
      {[9, 10, 11, 12, 18].map((slot, index) => renderSlot(slot, t('ui.miscValue', { value1: index + 1 })))}
    </div>
    <details className="equipment-special"><summary>{t('ui.warMachinesSpellbookReadOnly')}</summary>
      <div className="equipment-accessories">{[13, 14, 15, 16, 17].map(index => renderSlot(index, slots.find(slot => slot.slot === index)?.name ?? ''))}</div>
    </details>
    {selected && <div className="artifact-selection">
      <p><strong>{selected.name}: {selected.reservedBy ? t('ui.takenByValue2', { value1: selected.reservedBy }) : artifactLabel(selected.item)}</strong></p>
      {byId.get(selected.item.artifactId ?? -1)?.effect && <p className="fineprint">{byId.get(selected.item.artifactId ?? -1)?.effect}</p>}
      {reason ? <><p className="fineprint">{<ReasonText reason={reason} />}</p><button type="button" onClick={() => setTarget(null)}>{t('ui.close')}</button></> :
        <ArtifactPicker key={target} title={`${wornId !== null ? t('ui.change') : t('ui.equip')} · ${selected.name}`}
          options={options} onChoose={choose} onClose={() => setTarget(null)}>
          {wornId !== null && <div className="artifact-actions">
            <button type="button" disabled={!empty} onClick={() => {
              if (!empty) return;
              onApplyEdit({ equipmentSlot: { slot: selected.slot, item: { kind: 'empty' } },
                backpackSlot: { slot: empty.slot, item: { kind: 'artifact', value: wornId } } });
              setTarget(null);
            }}>{empty ? t('ui.moveToBackpack') : t('ui.backpackFull')}</button>
            <button type="button" onClick={() => {
              onApplyEdit({ equipmentSlot: { slot: selected.slot, item: { kind: 'empty' } } });
              setTarget(null);
            }}>{t('ui.remove')}</button>
          </div>}
        </ArtifactPicker>}
    </div>}
    <p className="fineprint">{t('ui.clickASquareToChangeTheArtifactInPlace')}</p>
  </section>;
}

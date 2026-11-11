import { ReasonText, type Reason } from '../i18n/reasons';
import { t, i18n } from '../i18n/index';
import { useEffect, useRef, useState } from 'react';
import { CreaturePortrait } from './CreaturePortrait';
import { CreatureCombobox } from './CreatureCombobox';
import { parseArmyCount } from '../lib/armyCount';
import type { ArmySlotDto, CreatureDto } from '../types/api';

/** Delay before a typed quantity becomes a pending edit; blur commits immediately. */
const COUNT_COMMIT_DELAY_MS = 350;

export function ArmyEditor({ slots, choices, readOnlyReason, dirtyFields, dirtyPrefix = 'Army slot', compact = false, allowEmpty = true, onApplyEdit }: {
  slots: ArmySlotDto[];
  choices: CreatureDto[];
  readOnlyReason: Reason | null;
  dirtyFields: Set<string>;
  /** Prefix of the dirty-field labels, "Army slot" for heroes and "Garrison slot" for towns. */
  dirtyPrefix?: string;
  compact?: boolean;
  allowEmpty?: boolean;
  onApplyEdit: (request: ArmyEditRequest) => void;
}) {
  return <>
    {!compact && readOnlyReason && <p className="fineprint">{<ReasonText reason={readOnlyReason} />}</p>}
    {!compact && !readOnlyReason && <p className="fineprint">{t('ui.armyChangesBecomePendingEditsImmediatelyNothingIsWritten')}</p>}
    <div className={`army-grid${compact ? ' army-grid-compact' : ''}`}>
      {slots.map((slot, index) => <ArmyStack key={index} slot={slot} index={index}
        choices={choices} compact={compact} allowEmpty={allowEmpty} dirty={dirtyFields.has(`${dirtyPrefix} ${index + 1}`)}
        onApplyEdit={onApplyEdit} />)}
    </div>
  </>;
}

/** Minimal request shape for an army/garrison slot edit (heroes and towns). */
export interface ArmyEditRequest {
  army?: { slot: number; creatureId: number | null; count: number };
}

function ArmyStack({ slot, index, choices, dirty, compact, allowEmpty, onApplyEdit }: {
  slot: ArmySlotDto;
  index: number;
  choices: CreatureDto[];
  dirty: boolean;
  compact: boolean;
  allowEmpty: boolean;
  onApplyEdit: (request: ArmyEditRequest) => void;
}) {
  const [creature, setCreature] = useState(slot.empty ? '' : String(slot.creatureId));
  const [count, setCount] = useState(slot.empty ? '' : String(slot.count));
  const countTimer = useRef<number | null>(null);
  // A successful edit, discard or reload supplies a fresh authoritative slot.
  useEffect(() => {
    setCreature(slot.empty ? '' : String(slot.creatureId));
    setCount(slot.empty ? '' : String(slot.count));
  }, [slot]);
  useEffect(() => () => {
    if (countTimer.current !== null) window.clearTimeout(countTimer.current);
  }, []);

  const dispatch = (creatureId: number | null, nextCount: number) => {
    onApplyEdit({ army: { slot: index, creatureId, count: nextCount } });
  };

  const onCreatureChange = (next: string) => {
    // A pending quantity commit must not overwrite the new selection.
    if (countTimer.current !== null) {
      window.clearTimeout(countTimer.current);
      countTimer.current = null;
    }
    setCreature(next);
    if (next === '') {
      // Clearing a stack commits count zero; keep an already-empty slot clean.
      if (!slot.empty) {
        setCount('');
        dispatch(null, 0);
      }
      return;
    }
    const id = Number(next);
    if (!slot.empty && id === slot.creatureId) return;
    if (slot.empty) {
      // First creature in an empty slot starts with a single unit.
      setCount('1');
      dispatch(id, 1);
    } else {
      // Replacing a creature keeps the current quantity when it is valid.
      dispatch(id, parseArmyCount(count) ?? slot.count);
    }
  };

  const commitCount = (text: string): boolean => {
    if (creature === '') return false;
    const parsed = parseArmyCount(text);
    if (parsed === null) return false;
    if (!slot.empty && parsed === slot.count) return false;
    dispatch(Number(creature), parsed);
    return true;
  };

  const onCountChange = (text: string) => {
    setCount(text);
    if (countTimer.current !== null) window.clearTimeout(countTimer.current);
    countTimer.current = window.setTimeout(() => {
      countTimer.current = null;
      commitCount(text);
    }, COUNT_COMMIT_DELAY_MS);
  };

  const onCountBlur = () => {
    if (countTimer.current !== null) {
      window.clearTimeout(countTimer.current);
      countTimer.current = null;
    }
    // Invalid or partial input stays uncommitted; restore the authoritative value.
    if (!commitCount(count)) setCount(slot.empty ? '' : String(slot.count));
  };

  return <div className={`army-stack${dirty ? ' army-stack-dirty' : ''}`} role="group"
    aria-label={t('ui.armySlotValue', { value1: index + 1 })}>
    <div className="army-stack-heading" title={t('ui.armySlotValue', { value1: index + 1 })}>{compact ? index + 1 : t('ui.armySlotValue', { value1: index + 1 })}
      {dirty && <span className="dirty-dot" title={t('ui.unsavedChange')} />}
      {!compact && !slot.editable && <span className="badge-readonly">{t('ui.readOnly')}</span>}
    </div>
    <div className="army-creature-row">
    {!compact && <CreaturePortrait key={creature} id={creature === '' ? null : Number(creature)} />}
    {slot.editable ? <>
      <div className="army-creature-field">
        {!compact && <span className="army-field-caption">{t('ui.creature')}</span>}
        <CreatureCombobox value={creature} choices={choices} allowEmpty={allowEmpty} onChange={onCreatureChange} />
      </div>
      <label className="army-quantity-field">
        {!compact && <span className="army-field-caption">{t('ui.quantity')}</span>}
        <input className="edit-number" type="number" min={1} max={4294967295} step={1}
          aria-label={t('ui.quantity')} disabled={creature === ''} value={count}
          onChange={(event) => onCountChange(event.target.value)}
          onBlur={onCountBlur} />
      </label>
    </> : <p className="army-creature-readonly">{slot.empty ? t('ui.empty') : `${slot.name ?? t('ui.creatureValue', { value1: slot.creatureId })} · ${slot.count.toLocaleString(i18n.resolvedLanguage ?? 'en')}`}</p>}
    </div>
    {!slot.editable && !slot.name && !slot.empty && <p className="fineprint">{t('ui.unrecognizedStackIsPreserved')}</p>}
  </div>;
}

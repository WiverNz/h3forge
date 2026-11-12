import { useEffect, useState } from 'react';
import { t } from '../i18n';
import { ReasonText, type ReasonDescriptor } from '../i18n/reasons';

export function CoordinatesEditor({ value, blocked, dirty, onApply }: {
  value: [number, number, number] | null;
  blocked: ReasonDescriptor | null;
  dirty: boolean;
  onApply: (to: [number, number, number]) => void;
}) {
  const [draft, setDraft] = useState<string[]>(value?.map(String) ?? ['', '', '']);
  const key = value?.join(',') ?? '';
  useEffect(() => { setDraft(key ? key.split(',') : ['', '', '']); }, [key]);
  const values = draft.map(Number);
  const valid = draft.every(s => s.trim() !== '') && values.every(v => Number.isInteger(v) && v >= 0 && v <= 251);
  const changed = values.some((v,i) => v !== value?.[i]);
  return <fieldset className={`hero-coordinates${dirty ? ' field-dirty' : ''}`} disabled={!!blocked || !value}>
    <legend>{t('ui.heroCoordinates')}{dirty ? ' *' : ''}</legend>
    {(['X','Y','Z'] as const).map((axis,i) => <label key={axis}>{axis}
      <input type="number" aria-label={axis} min={0} max={i === 2 ? 1 : 251}
        value={draft[i] ?? ''} placeholder="?" readOnly={i === 2}
        onChange={e => setDraft(draft.map((s,j) => j === i ? e.target.value : s))} />
    </label>)}
    <button type="button" disabled={!valid || !changed} onClick={() => onApply(values as [number,number,number])}>{t('ui.applyCoordinates')}</button>
    <small>{blocked ? <ReasonText reason={blocked} /> : t('ui.heroRelocationPolicy')}</small>
  </fieldset>;
}

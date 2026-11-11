import { t } from './index';
export interface ReasonDescriptor { code: string; details: string }
export type Reason = ReasonDescriptor | string;
export function reasonText(reason: Reason | null | undefined): string | undefined {
  if (reason == null) return undefined;
  if (typeof reason === 'string') return reason;
  return `${t('ui.readOnly')} · ${t('common.technicalDetails')}: ${reason.details}`;
}
export function ReasonText({ reason }: { reason: Reason }) {
  if (typeof reason === 'string') return <>{reason}</>;
  return <>{t('ui.readOnly')} <span className="feedback-details">{t('common.technicalDetails')}: {reason.details}</span></>;
}

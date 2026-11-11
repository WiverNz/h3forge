import { t } from './index';
import { errorSummary, type ErrorDescriptor } from './errors';
export type FeedbackMessage = ErrorDescriptor | { savedFile: string } | string;
export function Feedback({ message }: { message: FeedbackMessage }) {
  if (typeof message === 'string') return <>{message}</>;
  if ('savedFile' in message) return <>{t('common.saved', { file: message.savedFile })}</>;
  return <>{errorSummary(message)} <span className="feedback-details">
    {t('common.technicalDetails')}: <code>{message.code}</code> {message.message}
  </span></>;
}

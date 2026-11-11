import { t, formatNumber } from './index.ts';
import { en } from './locales/en.ts';
export interface ErrorDescriptor { code: string; message: string; params?: Record<string, number>; cause?: string }
export function describeError(error: unknown): ErrorDescriptor {
  if (typeof error === 'object' && error !== null && 'code' in error && typeof error.code === 'string') {
    const message = 'message' in error && typeof error.message === 'string' ? error.message : '';
    const params = 'params' in error && typeof error.params === 'object' && error.params !== null ? error.params as Record<string, number> : undefined;
    const cause = 'cause' in error && typeof error.cause === 'string' ? error.cause : undefined;
    return { code: error.code, message, params, cause };
  }
  return { code: 'unexpected', message: error instanceof Error ? error.message : String(error) };
}
export function errorSummary(error: ErrorDescriptor): string {
  if (error.code === 'open-failed') {
    const causes = { limit: 'messages.limit', 'unknown-format': 'messages.unknownFormat', 'unsupported-container': 'messages.unsupportedContainer', 'corrupt-input': 'messages.corrupt' } as const;
    if (error.cause && Object.hasOwn(causes, error.cause)) return t(causes[error.cause as keyof typeof causes]);
  }
  if (error.code === 'edit-failed' && error.cause === 'unsupported-format') return t('errors.edit-unsupported-format');
  if (error.code === 'edit-failed' && error.cause === 'unverified-record') return t('errors.hero-not-editable');
  if (error.code === 'edit-failed' && error.cause === 'invalid-value') return t('messages.invalidValue');
  if (error.code === 'edit-failed' && error.cause === 'stale-preimage') return t('messages.stale');
  switch (error.code) {
    case 'no-open-save': return t('common.noOpenSave');
    case 'grant-not-found': return t('common.grantNotFound');
    case 'source-changed': return t('common.sourceChanged');
    case 'nothing-to-save': return t('ui.noChangesToSaveEditASupportedFieldFirst');
    case 'save-disabled': return t('ui.valueUseSaveAsToWriteANewFile', { value1: t('ui.readOnly') });
    case 'file-too-large': {
      const limit = error.params?.limit;
      return typeof limit === 'number' && Number.isSafeInteger(limit) && limit >= 0
        ? t('common.fileTooLarge', { limit: formatNumber(limit) }) : t('common.genericError');
    }
    default: return Object.hasOwn(en.errors, error.code)
      ? t(`errors.${error.code as keyof typeof en.errors}`) : t('common.genericError');
  }
}

import { t } from '../i18n/index.ts';
import type { TextDto } from '../types/api';

/** Selectable legacy text decoding for stored save strings. */
export type LegacyEncoding = 'auto' | 'cp1251';

export const LEGACY_ENCODING_OPTIONS: { value: LegacyEncoding; label: string }[] = [
  { value: 'auto', get label() { return t('common.encodingAuto'); } },
  { value: 'cp1251', get label() { return t('common.encodingCyrillic'); } },
];

const UTF8_STRICT = new TextDecoder('utf-8', { fatal: true });
const CP1251 = new TextDecoder('windows-1251');

/**
 * Decodes raw save bytes for display without ever modifying them.
 * 'auto' tries strict UTF-8 first (ASCII and modern names pass through
 * unchanged) and falls back to Windows-1251, which is what the supplied
 * HotA save's Cyrillic hero names are stored in; 'cp1251' forces the
 * fallback for unusual saves.
 */
export function decodeRaw(raw: number[] | Uint8Array, encoding: LegacyEncoding): string {
  const bytes = raw instanceof Uint8Array ? raw : Uint8Array.from(raw);
  if (encoding === 'cp1251') {
    return CP1251.decode(bytes);
  }
  try {
    return UTF8_STRICT.decode(bytes);
  } catch {
    return CP1251.decode(bytes);
  }
}

/** Decodes a text DTO using its preserved raw bytes. */
export function decodeText(text: TextDto, encoding: LegacyEncoding): string {
  return decodeRaw(text.raw, encoding);
}

/** Compact hex dump for raw equipment/inventory words. */
export function toHex(bytes: number[]): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join(' ');
}

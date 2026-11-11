import { t } from './index';
const colorKeys = ['red', 'blue', 'tan', 'green', 'orange', 'purple', 'teal', 'pink'] as const;
const resourceKeys = ['wood', 'mercury', 'ore', 'sulfur', 'crystal', 'gems', 'gold'] as const;
export function colorName(color: number | null): string {
  const key = color === null ? undefined : colorKeys[color];
  return t(`colors.${key ?? 'neutral'}`);
}
export function resourceName(index: number): string {
  const key = resourceKeys[index];
  return key ? t(`resources.${key}`) : t('ui.unknown');
}

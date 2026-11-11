import { t } from './index';
export function dialogLabels(kind: 'open' | 'save' | 'map' | 'folder') {
  const titles = { open: 'common.openDialog', save: 'common.saveDialog', map: 'common.mapDialog', folder: 'common.folderDialog' } as const;
  return { labels: { title: t(titles[kind]), fileType: t(kind === 'map' ? 'common.mapsFilter' : 'common.savesFilter'), allFiles: t('common.allFiles') } };
}

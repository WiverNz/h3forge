import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppInfo } from '../types/api';

export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauri()) {
    throw new Error('Open the H3Forge desktop application to connect. This is a browser preview.');
  }
  return invoke<AppInfo>('get_app_info');
}

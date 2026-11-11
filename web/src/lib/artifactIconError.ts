/** Tauri rejects with strings or structured AppErrorDto values, not just Error. */
export function artifactIconError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
    const code = 'code' in error && typeof error.code === 'string' ? `${error.code}: ` : '';
    return `${code}${error.message}`;
  }
  return `Unexpected artifact icon error: ${String(error)}`;
}

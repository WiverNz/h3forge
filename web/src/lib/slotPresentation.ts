/** Only the first-word blank convention is verified (save/locate.rs). */
export function equipmentLabel(bytes: number[]): string {
  return bytes.length === 8 && bytes.slice(0, 4).every((byte) => byte === 0xff)
    ? 'Empty'
    : 'Artifact not identified';
}

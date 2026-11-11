import type { OwnerDto } from '../types/api';

// Binary owner IDs, matching definitions::names::PLAYER_COLORS.
const PLAYER_COLORS = ['red', 'blue', 'tan', 'green', 'orange', 'purple', 'teal', 'pink'] as const;
export function playerColor(owner: Pick<OwnerDto, 'kind' | 'player'>): string {
  return owner.kind === 'player' && owner.player !== null
    ? PLAYER_COLORS[owner.player] ?? 'neutral' : 'neutral';
}

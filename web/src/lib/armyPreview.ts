/** Rust supplies occupied stacks in slot order. Duplicate creatures are distinct stacks. */
export function armyPreview(creatureIds: readonly number[]) {
  return { icons: creatureIds.slice(0, 3), remaining: Math.max(0, creatureIds.length - 3) };
}

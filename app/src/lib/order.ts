/** Move the items at `picked` (indices) as one block so that the first of them lands at
 * 1-based position `pos`; everything else keeps its order and shifts out of the way. */
export function moveBlock<T>(list: T[], picked: number[], pos: number): T[] {
  const set = new Set(picked);
  const block = list.filter((_, i) => set.has(i));
  const rest = list.filter((_, i) => !set.has(i));
  const at = Math.min(Math.max(Math.round(pos) - 1, 0), rest.length);
  return [...rest.slice(0, at), ...block, ...rest.slice(at)];
}

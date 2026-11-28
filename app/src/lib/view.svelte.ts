// How the Live TV list is shown. Display only: the provider's own list is never changed.
type View = { channels: boolean; fold: boolean; lines: boolean };
const KEY = 'elara.view';
function load(): View {
  const d: View = { channels: true, fold: true, lines: true };
  try { return { ...d, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') }; } catch { return d; }
}
export const view = $state<View>(load());
export function saveView() { try { localStorage.setItem(KEY, JSON.stringify(view)); } catch {} }

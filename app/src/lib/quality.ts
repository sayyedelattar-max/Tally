// Which quality of a channel to use. Remembered per channel on this computer.
const KEY = 'elara.q';
const AUTO = 'elara.qauto';

function read<T>(k: string): Record<string, T> {
  try { return JSON.parse(localStorage.getItem(k) || '{}'); } catch { return {}; }
}
function write(k: string, v: unknown) {
  try { localStorage.setItem(k, JSON.stringify(v)); } catch {}
}

/** The quality (channel id) last chosen for this channel, if any. */
export const qGet = (key: string): number | undefined => read<number>(KEY)[key];
export function qSet(key: string, id: number) {
  const m = read<number>(KEY);
  m[key] = id;
  write(KEY, m);
}
/** Auto is the default: start well and step down by itself when the stream struggles. */
export const autoGet = (key: string): boolean => read<boolean>(AUTO)[key] !== false;
export function autoSet(key: string, on: boolean) {
  const m = read<boolean>(AUTO);
  m[key] = on;
  write(AUTO, m);
}

import type { Alt, Chan } from './backend';
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

/** One entry per channel from a merged group: the remembered (or best non-HEVC) quality is the one that plays. */
export function asRow(m: { key: string; name: string; selected?: number; variants: { channel: Chan; label: string; score: number; hevc: boolean }[] }): Chan {
  const alts: Alt[] = m.variants.map((v) => ({ chan: v.channel, label: v.label, score: v.score, hevc: v.hevc }));
  const want = qGet(m.key);
  const pick = alts.find((a) => a.chan.id === want) ?? alts.find((a) => a.chan.id === m.selected) ?? alts.find((a) => !a.hevc) ?? alts[0];
  return alts.length > 1 ? { ...pick.chan, name: m.name, fav: alts.some((a) => a.chan.fav), alts, mkey: m.key } : pick.chan;
}

/** The best picture on offer, read from the provider's own words (4K, FHD, HD). */
export function bestTag(c: Chan): string {
  const words = (c.alts ?? []).flatMap((a) => a.label.toUpperCase().split(/\s+/));
  for (const [tag, re] of [['4K', /^(4K|UHD|2160P|8K)$/], ['FHD', /^(FHD|1080P|POWER|MAX)$/], ['HD', /^(HD|720P)$/]] as const) if (words.some((w) => re.test(w))) return tag;
  return '';
}

// Bridge to the Rust side. In a plain browser (design preview) `inApp` is false and nothing here is called.

export const inApp = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export type PlaylistInfo = {
  id: string;
  name: string;
  host: string;
  user: string;
  status: string;
  expiresAt: number | null;
  trial: boolean;
  maxConnections: number | null;
  activeConnections: number | null;
  channels: number;
  updatedAt: number;
};
export type Registry = { active: string | null; items: PlaylistInfo[] };
export type Group = { name: string; count: number; hidden: boolean };
export type VodRef = { kind: 'movie' | 'episode'; id: string; ext: string };
export type Alt = { chan: Chan; label: string; score: number; hevc: boolean };
export type Chan = { alts?: Alt[]; mkey?: string; id: number; tvgId: string; name: string; group: string; logo: string; number: number | null; fav: boolean; hidden?: boolean; vod?: VodRef };
export type TitleItem = { kind: 'movie' | 'series'; id: string; name: string; group: string; poster: string; plot: string; rating: number; year: string; ext: string };
export type Shelf = { name: string; recent: boolean; count: number; items: TitleItem[] };
export type EpisodeItem = { id: string; season: number; number: number; title: string; plot: string; duration: string; ext: string };
export type SeriesInfo = { name: string; plot: string; episodes: EpisodeItem[] };
export type HomeData = { lastLive: Chan | null; nowTitle: string; nextTitle: string; nowStart: number; nowStop: number; recentLive: Chan[]; favorites: Chan[]; continueTitles: TitleItem[]; newMovies: TitleItem[]; newSeries: TitleItem[]; liveCount: number; movieCount: number; seriesCount: number };
export type Prog = { start: number; stop: number; title: string; desc: string };
export type GuideRow = { channel: Chan; progs: Prog[] };
export type ChanPage = { rows: Chan[]; next: [number, number] | null };

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export const api = {
  playlists: () => call<Registry>('playlists_list'),
  save: (a: { id: string | null; name: string; host: string; user: string; pass: string | null }) => call<PlaylistInfo>('playlist_save', a),
  refresh: (id: string) => call<PlaylistInfo>('playlist_refresh', { id }),
  remove: (id: string) => call<void>('playlist_remove', { id }),
  activate: (id: string) => call<void>('playlist_activate', { id }),
  groups: (showHidden = false) => call<Group[]>('live_groups', { showHidden }),
  channelsSorted: (a: { group: string | null; mode: string; showHidden: boolean; offset: number; limit: number }) => call<Chan[]>('live_channels_sorted', a),
  reorderChannels: (ids: number[]) => call<void>('reorder_channels', { ids }),
  setHidden: (id: number, hidden: boolean) => call<void>('set_hidden', { id, hidden }),
  reorderGroups: (names: string[]) => call<void>('reorder_groups', { names }),
  setGroupHidden: (name: string, hidden: boolean) => call<void>('set_group_hidden', { name, hidden }),
  channels: (a: { group: string | null; query: string | null; after: [number, number] | null; limit: number }) => call<ChanPage>('live_channels', a),
  diagnose: (id: number) => call<string>('diagnose_channel', { id }),
  play: (id: number) => call<{ url: string }>('play_channel', { id }),
  hqAvailable: () => call<boolean>('hq_available'),
  hqPlay: (items: { name: string; liveId: number | null; vod: VodRef | null; alts: { liveId: number; label: string }[]; alt: number }[], index: number, options: { quality: string; bufferSecs: number; fullscreen: boolean }) =>
    call<void>('hq_play', { items, index, options }),
  hqStop: () => call<void>('hq_stop'),
  shelves: (kind: string, perRow: number, maxRows: number) => call<Shelf[]>('title_rows', { kind, perRow, maxRows }),
  titles: (a: { kind: string; group: string | null; query: string | null; offset: number; limit: number }) => call<TitleItem[]>('title_page', a),
  series: (id: string) => call<SeriesInfo>('series_detail', { id }),
  playTitle: (v: VodRef) => call<{ url: string }>('play_title', { kind: v.kind, id: v.id, ext: v.ext }),
  merged: (group: string | null, query: string | null, showHidden: boolean) => call<{ key: string; name: string; variants: { channel: Chan; label: string; score: number; hevc: boolean }[] }[]>('live_channels_merged', { group, query, showHidden }),
  bulk: (ids: number[], action: 'favorite' | 'unfavorite' | 'hide' | 'unhide' | 'move', group?: string) => call<void>('bulk_channels', { ids, action, group: group ?? null }),
  guideStatus: () => call<{ count: number; newest: number }>('guide_status'),
  guideSync: () => call<number>('guide_sync'),
  guideRows: (source: string, from: number, to: number, offset: number, limit: number) => call<GuideRow[]>('guide_rows', { source, from, to, offset, limit }),
  recordWatch: (kind: 'live' | 'movie' | 'series', id: string) => call<void>('record_watch', { kind, id }),
  home: () => call<HomeData>('home_data'),
  favorites: () => call<Chan[]>('live_favorites'),
  reorder: (ids: number[]) => call<void>('reorder_favorites', { ids }),
  setFavorite: (id: number, fav: boolean) => call<void>('set_favorite', { id, fav }),
};

/** Subscription end as a date and whole days left (negative once expired). */
export function expiry(epoch: number | null): { date: string; days: number } | null {
  if (epoch === null) return null;
  const d = new Date(epoch * 1000);
  const date = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  return { date, days: Math.floor((epoch * 1000 - Date.now()) / 86_400_000) };
}

/** Events from the high-quality player window. Returns a function that stops listening. */
export async function onHq(handlers: { closed?: () => void; index?: (i: number) => void; error?: (m: string) => void; alt?: (index: number, liveId: number) => void }): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  const offs = await Promise.all([
    listen('hq-closed', () => handlers.closed?.()),
    listen<number>('hq-index', (e) => handlers.index?.(e.payload)),
    listen<string>('hq-error', (e) => handlers.error?.(e.payload)),
    listen<[number, number]>('hq-alt', (e) => handlers.alt?.(e.payload[0], e.payload[1])),
  ]);
  return () => offs.forEach((f) => f());
}

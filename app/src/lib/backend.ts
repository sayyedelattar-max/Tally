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
export type Group = { name: string; count: number };
export type Chan = { id: number; tvgId: string; name: string; group: string; logo: string; number: number | null; fav: boolean };
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
  groups: () => call<Group[]>('live_groups'),
  channels: (a: { group: string | null; query: string | null; after: [number, number] | null; limit: number }) => call<ChanPage>('live_channels', a),
  play: (id: number) => call<{ url: string }>('play_channel', { id }),
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

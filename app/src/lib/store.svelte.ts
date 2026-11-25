import type { Accent, Channel, Lang, Playing, Prog, Title, View } from './types';
import { dict } from './i18n';

import { api, inApp, type Chan, type PlaylistInfo } from './backend';

/** All UI state in one place. Mutate fields directly; components react to what they read. */
export const ui = $state({
  /** The channel being watched in the app, and the list it was picked from (for zapping). */
  rplay: null as null | { list: Chan[]; index: number },
  real: { loaded: false, active: null as string | null, items: [] as PlaylistInfo[] },
  view: 'home' as View,
  lang: 'en' as Lang,
  accent: 'mono' as Accent,
  searchOpen: false,
  playing: null as Playing | null,
  channelId: 201,
  prevChannelId: 101,
  favs: [101, 201, 301, 401, 601, 701] as number[],
  recording: [] as number[],
  multi: [201, 101, 301, 601] as number[],
  multiFocus: 0,
  detail: null as string | null,
  hidden: { live: [] as string[], movies: [] as string[], series: [] as string[] },
  settings: { format: 'ts', hw: true, buffer: 'balanced', engine: 'auto', quality: 'high', subSize: 'medium', parental: false },
  toast: '',
  activePlaylist: 1,
});

export function t(key: string): string {
  return dict[ui.lang][key] ?? dict.en[key] ?? key;
}

export const locale = () => (ui.lang === 'ar' ? 'ar-EG' : 'en-GB');
export const chName = (c: Channel) => (ui.lang === 'ar' && c.nameAr ? c.nameAr : c.name);
export const progTitle = (p: Prog) => (ui.lang === 'ar' && p.titleAr ? p.titleAr : p.title);
export const titleName = (x: Title) => x.title;

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(msg: string) {
  ui.toast = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (ui.toast = ''), 2400);
}

export function go(view: View) {
  ui.view = view;
  ui.detail = null;
}

export function openTitle(x: Title) {
  ui.view = x.kind === 'movie' ? 'movies' : 'series';
  ui.detail = x.id;
}

export function tune(id: number) {
  if (id !== ui.channelId) {
    ui.prevChannelId = ui.channelId;
    ui.channelId = id;
  }
}

export function toggleFav(id: number) {
  const i = ui.favs.indexOf(id);
  if (i >= 0) ui.favs.splice(i, 1);
  else ui.favs.push(id);
}

export function toggleRecord(id: number) {
  const i = ui.recording.indexOf(id);
  if (i >= 0) {
    ui.recording.splice(i, 1);
    toast(t('toast.recStopped'));
  } else {
    ui.recording.push(id);
    toast(t('toast.recStarted'));
  }
}

let lastFocus: HTMLElement | null = null;

export function play(p: Playing) {
  lastFocus = document.activeElement as HTMLElement | null;
  if (p.kind === 'live') tune(p.id);
  ui.playing = p;
}

export function stop() {
  ui.playing = null;
  queueMicrotask(() => lastFocus?.focus?.());
}

export function openSearch() {
  lastFocus = document.activeElement as HTMLElement | null;
  ui.searchOpen = true;
}

export function closeSearch() {
  ui.searchOpen = false;
  queueMicrotask(() => lastFocus?.focus?.());
}

/** Reads the saved playlists from the app. Call again after any change. */
export async function loadRegistry() {
  if (!inApp) return;
  try {
    const r = await api.playlists();
    ui.real.active = r.active;
    ui.real.items = r.items;
  } finally {
    ui.real.loaded = true;
  }
}

export const realActive = () => ui.real.items.find((p) => p.id === ui.real.active) ?? null;

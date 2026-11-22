export type Lang = 'en' | 'ar';
export type View = 'home' | 'live' | 'movies' | 'series' | 'replay' | 'playlists' | 'settings';
export type Accent = 'mono' | 'teal' | 'violet' | 'amber';

/** Programme times are minutes relative to "now" (negative = already aired). */
export interface Prog {
  start: number;
  end: number;
  title: string;
  titleAr?: string;
}

export interface Channel {
  id: number;
  num: number;
  code: string;
  name: string;
  nameAr?: string;
  cat: string;
  catchup: boolean;
  progs: Prog[];
}

export interface Episode {
  n: number;
  title: string;
  mins: number;
  progress?: number;
}

export interface Title {
  id: string;
  kind: 'movie' | 'series';
  title: string;
  year: number;
  cat: string;
  mins?: number;
  seasons?: Episode[][];
  rating: string;
  synopsis: string;
  progress?: number;
  added: number;
}

export interface Playlist {
  id: number;
  name: string;
  type: 'xtream' | 'm3u';
  host: string;
  status: string;
  expires: string | null;
  connections: [number, number] | null;
  counts: { live: number; movies: number; series: number };
  updated: string;
}

export type Playing =
  | { kind: 'live'; id: number }
  | { kind: 'replay'; id: number; start: number }
  | { kind: 'vod'; id: string; season?: number; ep?: number };

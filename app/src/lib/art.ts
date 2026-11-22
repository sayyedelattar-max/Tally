// Procedural artwork for the prototype: flat colour fields with a few geometric
// shapes, like a printed poster. No gradients, no stock imagery, no real titles.
// In the app, real posters and channel logos from the provider replace this.

export type Shape =
  | { t: 'circle'; cx: number; cy: number; r: number; fill?: string; stroke?: string; sw?: number }
  | { t: 'rect'; x: number; y: number; w: number; h: number; fill: string }
  | { t: 'path'; d: string; fill: string };

// [background, shape A, shape B, ink]. Muted mid-tones with one warm or cool pop.
const PALETTES: [string, string, string, string][] = [
  ['#26324a', '#e6b565', '#d4553f', '#f2eadb'],
  ['#1d3a35', '#d7c3a1', '#e07a5f', '#f1ece0'],
  ['#3a2a3f', '#f2b8a2', '#8fb8a8', '#f4ece6'],
  ['#40322a', '#8fb3c9', '#e8d39b', '#f3ece2'],
  ['#22303c', '#b8d4c2', '#ef8354', '#eef0ea'],
  ['#3b2430', '#d9c27f', '#7ea3b8', '#f3e9e4'],
  ['#2c3a2b', '#e0a458', '#b7cfe0', '#f0eddc'],
  ['#2b2b3d', '#e5989b', '#a3c4a0', '#f1ebe6'],
];

export function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export interface ArtSpec {
  bg: string;
  a: string;
  b: string;
  ink: string;
  comp: number;
  r: number;
}

export function artFor(seed: string): ArtSpec {
  const h = hash(seed);
  const p = PALETTES[h % PALETTES.length];
  return { bg: p[0], a: p[1], b: p[2], ink: p[3], comp: (h >>> 8) % 6, r: ((h >>> 16) % 100) / 100 };
}

export function shapes(comp: number, w: number, h: number, a: string, b: string, r: number): Shape[] {
  const m = Math.min(w, h);
  const sx = w * (0.55 + r * 0.25);
  switch (comp) {
    case 0:
      return [
        { t: 'circle', cx: sx, cy: h * 0.34, r: m * 0.3, fill: a },
        { t: 'rect', x: 0, y: h * 0.76, w, h: h * 0.06, fill: b },
      ];
    case 1:
      return [
        { t: 'circle', cx: w * 0.4, cy: h * 0.38, r: m * 0.28, fill: a },
        { t: 'circle', cx: w * 0.62, cy: h * 0.46, r: m * 0.2, fill: b },
      ];
    case 2:
      return [
        { t: 'rect', x: 0, y: h * 0.14, w: w * 0.78, h: h * 0.09, fill: a },
        { t: 'rect', x: 0, y: h * 0.27, w: w * 0.56, h: h * 0.09, fill: b },
        { t: 'rect', x: 0, y: h * 0.4, w: w * 0.34, h: h * 0.09, fill: a },
      ];
    case 3:
      return [
        { t: 'path', d: `M0 ${h} L${w} ${h * 0.4} L${w} ${h} Z`, fill: a },
        { t: 'circle', cx: w * 0.3, cy: h * 0.3, r: m * 0.17, fill: b },
      ];
    case 4: {
      const x0 = w * (0.3 + r * 0.1);
      const x1 = x0 + w * 0.34;
      const rad = (x1 - x0) / 2;
      return [
        { t: 'path', d: `M${x0} ${h * 0.78} V${h * 0.34 + rad * 0} A${rad} ${rad} 0 0 1 ${x1} ${h * 0.34} V${h * 0.78} Z`, fill: a },
        { t: 'circle', cx: x0 + rad, cy: h * 0.5, r: m * 0.08, fill: b },
      ];
    }
    default:
      return [
        { t: 'circle', cx: w * 0.62, cy: h * 0.4, r: m * 0.34, stroke: a, sw: m * 0.035 },
        { t: 'circle', cx: w * 0.62, cy: h * 0.4, r: m * 0.22, stroke: b, sw: m * 0.035 },
        { t: 'circle', cx: w * 0.62, cy: h * 0.4, r: m * 0.1, fill: a },
      ];
  }
}

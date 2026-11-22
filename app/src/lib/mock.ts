// Fictional sample data for the prototype. Every channel, title and provider here is
// invented. The real app reads the same shapes from tally-core.

import type { Channel, Episode, Playlist, Prog, Title } from './types';

/** Fixed "now" so screenshots and tests are stable: Saturday 21 Nov 2026, 21:10. */
export const TODAY = new Date(2026, 10, 21, 21, 10);
export const CLOCK0 = 21 * 60 + 10;

const pad = (n: number) => String(n).padStart(2, '0');

export function clock(min: number): string {
  const t = (((CLOCK0 + min) % 1440) + 1440) % 1440;
  return `${pad(Math.floor(t / 60))}:${pad(t % 60)}`;
}

/** 0 = today, -1 = yesterday ... for a minute offset relative to now. */
export const dayIndex = (min: number) => Math.floor((CLOCK0 + min) / 1440);
/** Minute offset (relative to now) at which day `d` starts. */
export const dayStart = (d: number) => d * 1440 - CLOCK0;

export function dayDate(d: number): Date {
  const x = new Date(TODAY);
  x.setDate(x.getDate() + d);
  return x;
}

export function fmtDuration(mins: number, lang: 'en' | 'ar'): string {
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  if (lang === 'ar') return h ? `${h} س ${m ? m + ' د' : ''}`.trim() : `${m} د`;
  return h ? `${h}h ${m ? pad(m) + 'm' : ''}`.trim() : `${m}m`;
}

// ---------------------------------------------------------------- live TV

const PROG_TITLES: Record<string, [string, string][]> = {
  news: [
    ['Morning Brief', 'موجز الصباح'], ['Harbor Hour', 'ساعة المرفأ'], ['Markets Today', 'أسواق اليوم'],
    ['World Desk', 'المكتب الدولي'], ['Evening Bulletin', 'نشرة المساء'], ['The Long Table', 'المائدة الطويلة'],
    ['Late Edition', 'الطبعة المتأخرة'], ['Weather & Roads', 'الطقس والطرق'],
  ],
  sport: [
    ['Derby Night Live', 'ليلة الديربي مباشر'], ['Pre-match Studio', 'استوديو ما قبل المباراة'], ['Full-time Report', 'تقرير نهاية المباراة'],
    ['Rally Stage Replay', 'إعادة مرحلة الراليات'], ['Court Side: Semifinal', 'على حافة الملعب: نصف النهائي'], ['Training Ground Weekly', 'ملعب التدريب الأسبوعي'],
    ['Highlights Review', 'مراجعة الملخصات'], ['Transfer Talk', 'حديث الانتقالات'],
  ],
  movies: [
    ['The Salt Road', 'طريق الملح'], ['Night Shift', 'وردية الليل'], ['Paper Moons', 'أقمار من ورق'],
    ['Wind Tower', 'برج الريح'], ['Trailers & Previews', 'إعلانات وعروض'], ['Saffron Lane', 'زقاق الزعفران'],
    ['Black Compass', 'البوصلة السوداء'], ['Tin Orchard', 'بستان الصفيح'],
  ],
  kids: [
    ['Bedtime Stories', 'حكايات قبل النوم'], ['Robo Gardeners', 'البستانيون الآليون'], ['Little Lighthouse', 'المنارة الصغيرة'],
    ['Toy Bay Club', 'نادي خليج الألعاب'], ['Kite Adventures', 'مغامرات الطائرة الورقية'], ['Sing Along', 'غنّوا معنا'],
  ],
  music: [
    ['Delta Top 20', 'دلتا أفضل ٢٠'], ['Live Sessions: Oud & Strings', 'جلسات حية: عود ووتريات'], ['Retro Cassette', 'شريط كاسيت قديم'],
    ['Night Drive Mix', 'ميكس قيادة الليل'], ['Unplugged', 'بدون كهرباء'], ['Request Hour', 'ساعة الطلبات'],
  ],
  docs: [
    ['Deep Ocean Cities', 'مدن أعماق المحيط'], ['The Last Typesetters', 'آخر صنّاع الحروف'], ['Salt, Wind, Stone', 'ملح ورياح وحجر'],
    ['Night Atlas', 'أطلس الليل'], ['River Without End', 'نهر بلا نهاية'], ['Dust and Stars', 'غبار ونجوم'],
  ],
  drama: [
    ['Oasis Evenings', 'أمسيات الواحة'], ['House of Seven Doors', 'بيت الأبواب السبعة'], ['The Café Years', 'سنوات المقهى'],
    ['Letters from Alexandria', 'رسائل من الإسكندرية'], ['Rerun Hour', 'ساعة الإعادة'], ['Second Spring', 'ربيع ثانٍ'],
  ],
};

const rnd = (n: number) => Math.imul(n ^ 0x9e3779b9, 2654435761) >>> 0;

function genProgs(ci: number, cat: string): Prog[] {
  const titles = PROG_TITLES[cat];
  const durs = [30, 45, 60, 60, 90, 120];
  const out: Prog[] = [];
  let t = -4500 - ((ci * 13) % 50);
  let k = ci * 3;
  while (t < 1500) {
    const d = durs[rnd(k + ci * 101) % durs.length];
    const [en, ar] = titles[rnd(k * 31 + ci) % titles.length];
    out.push({ start: t, end: t + d, title: en, titleAr: ar });
    t += d;
    k++;
  }
  return out;
}

const CH: [number, string, string, string, string, boolean][] = [
  [101, 'HN', 'Harbor News 24', 'المرفأ نيوز ٢٤', 'news', true],
  [102, 'FW', 'Falcon Weather', 'الصقر للطقس', 'news', false],
  [103, 'GB', 'Gulf Business', 'الخليج للأعمال', 'news', true],
  [104, 'WD', 'World Desk', 'المكتب الدولي', 'news', true],
  [201, 'MS1', 'Meridian Sports 1', 'ميريديان سبورت ١', 'sport', true],
  [202, 'MS2', 'Meridian Sports 2', 'ميريديان سبورت ٢', 'sport', true],
  [203, 'AR', 'Arena Live', 'الحلبة مباشر', 'sport', true],
  [204, 'MT', 'Motor Tracks', 'مسارات المحركات', 'sport', false],
  [301, 'CQ', 'Cinema Quarter', 'حي السينما', 'movies', true],
  [302, 'LC', 'Lantern Classics', 'فانوس الكلاسيكيات', 'movies', true],
  [303, 'NS', 'Night Shift Cinema', 'سينما الوردية', 'movies', false],
  [304, 'ID', 'Indie Door', 'الباب المستقل', 'movies', true],
  [401, 'AD', 'Atlas Docs', 'أطلس الوثائقية', 'docs', true],
  [402, 'WN', 'Wild Nile', 'النيل البري', 'docs', true],
  [403, 'DS', 'Deep Space', 'الفضاء العميق', 'docs', false],
  [501, 'KK', 'Kite Kids', 'قناة الطائرة الورقية', 'kids', true],
  [502, 'LL', 'Little Lighthouse', 'المنارة الصغيرة', 'kids', true],
  [503, 'TB', 'Toy Bay', 'خليج الألعاب', 'kids', false],
  [601, 'DM', 'Delta Music', 'دلتا ميوزيك', 'music', true],
  [602, 'OU', 'Oud & Strings', 'عود ووتريات', 'music', true],
  [603, 'RC', 'Retro Cassette', 'شريط كاسيت', 'music', false],
  [701, 'OD', 'Oasis Drama', 'واحة الدراما', 'drama', true],
  [702, 'SD', 'Seven Doors', 'الأبواب السبعة', 'drama', true],
  [703, 'CY', 'Café Years', 'سنوات المقهى', 'drama', true],
];

export const channels: Channel[] = CH.map(([num, code, name, nameAr, cat, catchup], i) => ({
  id: num, num, code, name, nameAr, cat, catchup, progs: genProgs(i, cat),
}));

export const channelById = (id: number) => channels.find((c) => c.id === id)!;
export const nowOn = (c: Channel) => c.progs.find((p) => p.start <= 0 && 0 < p.end)!;
export const nextOn = (c: Channel) => c.progs.find((p) => p.start >= nowOn(c).end)!;
export const progress = (p: Prog) => Math.min(1, Math.max(0, (0 - p.start) / (p.end - p.start)));

export const liveCategories = ['news', 'sport', 'movies', 'docs', 'kids', 'music', 'drama'];

/** Programmes starting within the next `within` minutes, soonest first. */
export function startingSoon(within = 75, limit = 5) {
  const out: { ch: Channel; p: Prog }[] = [];
  for (const ch of channels) for (const p of ch.progs) if (p.start > 0 && p.start <= within) out.push({ ch, p });
  return out.sort((a, b) => a.p.start - b.p.start).slice(0, limit);
}

// ----------------------------------------------------------- movies and series

const mv = (id: string, title: string, year: number, cat: string, mins: number, rating: string, synopsis: string, added: number, progress?: number): Title =>
  ({ id, kind: 'movie', title, year, cat, mins, rating, synopsis, added, progress });

export const movies: Title[] = [
  mv('m1', 'Salt Road', 2026, 'Drama', 116, 'PG-13', 'A caravan guide and a runaway heir cross the dry coast to reach a port that may no longer exist.', 1, 0.38),
  mv('m2', 'Wind Tower', 2026, 'Thriller', 108, 'PG-13', 'A structural engineer discovers that the building she certified is slowly being moved.', 2),
  mv('m3', 'Copper Season', 2025, 'Drama', 124, 'R', 'Three generations argue over a failing foundry during its final harvest of orders.', 3),
  mv('m4', 'Paper Moons', 2025, 'Comedy', 91, 'PG', 'Two failing lantern makers fake a festival to save their street.', 4),
  mv('m5', 'The Quiet Dock', 2026, 'Thriller', 116, 'R', 'A night watchman logs a ship that the harbour office swears never arrived.', 5, 0.72),
  mv('m6', 'Night Ferry', 2024, 'Thriller', 102, 'PG-13', 'The last ferry of the night carries one passenger too many.', 6),
  mv('m7', 'Glass Harvest', 2025, 'Sci-Fi', 130, 'PG-13', 'On a farm-ship, the final crop begins answering questions.', 7),
  mv('m8', 'Orbit Café', 2026, 'Comedy', 97, 'PG', 'The only café on a space station loses its only barista and gains a rival.', 8),
  mv('m9', 'Ninth Lighthouse', 2024, 'Family', 94, 'G', 'A girl and her grandfather restore the ninth lighthouse on a forgotten coast.', 9),
  mv('m10', 'Dust Parade', 2023, 'Action', 118, 'R', 'A convoy driver has until sunrise to cross a desert that is rearranging itself.', 10),
  mv('m11', 'Harbor Kings', 2025, 'Action', 112, 'R', 'Rival tug crews race to win the contract that will rebuild the port.', 11),
  mv('m12', 'Stillwater', 2024, 'Drama', 105, 'PG-13', 'A swimming coach returns to the town that stopped believing her.', 12),
  mv('m13', 'Midnight Tailor', 2026, 'Thriller', 99, 'PG-13', 'Every suit he stitches predicts how its owner will disappear.', 13),
  mv('m14', 'Cedar & Ash', 2023, 'Drama', 121, 'PG-13', 'Two brothers inherit a forest and a decades-old promise.', 14),
  mv('m15', 'The Long Tide', 2022, 'Drama', 137, 'PG-13', 'A fishing village counts what the sea has taken and what it quietly returned.', 15),
  mv('m16', 'Red Kite', 2026, 'Family', 88, 'G', 'A kite maker teaches a stubborn wind to cooperate.', 16),
  mv('m17', 'Atlas Run', 2025, 'Action', 109, 'PG-13', 'A courier crosses four borders in one day with a package that keeps getting heavier.', 17),
  mv('m18', 'Borrowed Sky', 2024, 'Sci-Fi', 126, 'PG-13', 'A city on stilts rents its weather from the neighbours.', 18),
  mv('m19', 'Marble Heist', 2023, 'Comedy', 101, 'PG-13', 'A museum guard and a retired thief swap jobs for one weekend.', 19),
  mv('m20', 'Echo Station', 2026, 'Sci-Fi', 114, 'R', 'A relay operator hears her own voice arriving from next week.', 20),
  mv('m21', 'Little Embers', 2025, 'Family', 92, 'G', 'A young dragon keeper learns that small fires need the most care.', 21),
  mv('m22', 'Saffron Lane', 2022, 'Comedy', 96, 'PG', 'A spice market feud is settled by the world’s worst cooking show.', 22),
  mv('m23', 'Black Compass', 2024, 'Action', 123, 'R', 'A navigator with a broken instrument leads a rescue she cannot map.', 23),
  mv('m24', 'Tin Orchard', 2023, 'Drama', 108, 'PG-13', 'A widow rebuilds an orchard from scrap, one stubborn tree at a time.', 24),
];

const EP_TITLES = ['Arrival', 'The Offer', 'Low Water', 'Paper Walls', 'Second Key', 'Open Market', 'The Understudy', 'Last Ferry', 'Quiet Hours', 'Rising Dust'];

function seasons(id: string, counts: number[], watched: [number, number, number?]): Episode[][] {
  return counts.map((n, s) =>
    Array.from({ length: n }, (_, e) => {
      const ep: Episode = { n: e + 1, title: EP_TITLES[(rnd(hash2(id) + s * 17 + e) + e) % EP_TITLES.length], mins: 38 + (rnd(s * 31 + e + hash2(id)) % 18) };
      if (s < watched[0] || (s === watched[0] && e < watched[1])) ep.progress = 1;
      else if (s === watched[0] && e === watched[1] && watched[2] !== undefined) ep.progress = watched[2];
      return ep;
    }),
  );
}
const hash2 = (s: string) => [...s].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);

const sr = (id: string, title: string, year: number, cat: string, rating: string, synopsis: string, counts: number[], added: number, watched: [number, number, number?], progress?: number): Title =>
  ({ id, kind: 'series', title, year, cat, rating, synopsis, added, progress, seasons: seasons(id, counts, watched) });

export const series: Title[] = [
  sr('s1', 'Oasis Evenings', 2023, 'Drama', 'PG-13', 'Six households share one courtyard, one well and one very old argument.', [10, 10, 8], 1, [2, 3, 0.4], 0.45),
  sr('s2', 'Sixth Floor', 2025, 'Thriller', 'R', 'A building’s sixth floor appears on no plan, and the lift keeps stopping there.', [8, 8], 2, [0, 0]),
  sr('s3', 'Deep Ocean Cities', 2024, 'Docuseries', 'G', 'Engineers and divers map the settlements that grew on the continental shelf.', [6, 6], 3, [0, 2, 0.8], 0.2),
  sr('s4', 'Copper Street', 2022, 'Drama', 'PG-13', 'The last working metal market in the old quarter fights its landlord.', [10, 10, 10, 8], 4, [0, 0]),
  sr('s5', 'Harbor Nights', 2025, 'Crime', 'R', 'A night-shift detective works the docks between the last ship and the first.', [8, 8], 5, [0, 0]),
  sr('s6', 'Station Eleven Hundred', 2026, 'Sci-Fi', 'PG-13', 'Maintenance crews on an orbital ring keep finding rooms nobody built.', [8], 6, [0, 0]),
  sr('s7', 'The Understudies', 2024, 'Comedy', 'PG', 'A theatre’s backup cast takes over after the stars all fall ill at once.', [10, 10], 7, [0, 0]),
  sr('s8', 'Little Lighthouse', 2025, 'Kids', 'G', 'A tiny lighthouse and her friends guide lost boats home.', [12, 12], 8, [0, 0]),
  sr('s9', 'Wild Nile', 2023, 'Docuseries', 'G', 'A year along the river, source to delta, one season at a time.', [6, 6, 6], 9, [0, 0]),
  sr('s10', 'Quarter Past Nine', 2026, 'Comedy', 'PG-13', 'A radio station’s night team improvises through every broadcast disaster.', [8], 10, [0, 0]),
  sr('s11', 'Saltwater Law', 2024, 'Drama', 'PG-13', 'A maritime court on an island that sinks a little every year.', [10, 8], 11, [0, 0]),
  sr('s12', 'Kite Club', 2025, 'Kids', 'G', 'Four neighbours build a kite for every kind of weather.', [10, 10], 12, [0, 0]),
];

export const allTitles = [...movies, ...series];
export const titleById = (id: string) => allTitles.find((t) => t.id === id)!;
export const movieCategories = [...new Set(movies.map((m) => m.cat))];
export const seriesCategories = [...new Set(series.map((s) => s.cat))];

// -------------------------------------------------------------------- playlists

export const playlists: Playlist[] = [
  { id: 1, name: 'Home subscription', type: 'xtream', host: 'provider.example', status: 'Active', expires: '2027-03-02', connections: [0, 2], counts: { live: 24, movies: 24, series: 12 }, updated: '20:41' },
  { id: 2, name: 'Sports pack', type: 'm3u', host: 'cdn.sports.example', status: 'Active', expires: null, connections: null, counts: { live: 8, movies: 0, series: 0 }, updated: 'Yesterday' },
];

export function daysLeft(iso: string | null): number | null {
  if (!iso) return null;
  const [y, m, d] = iso.split('-').map(Number);
  return Math.round((new Date(y, m - 1, d).getTime() - new Date(2026, 10, 21).getTime()) / 86400000);
}

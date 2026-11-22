// Mock-only mirror of crates/tally-core/src/norm.rs and the typo logic in store.rs,
// so the prototype searches exactly like the real core will. In the app this whole
// file is replaced by a call into Rust.

export function normalize(input: string): string {
  let out = '';
  let pending = false;
  for (const ch of input) {
    const cp = ch.codePointAt(0)!;
    if ((cp >= 0x064b && cp <= 0x065f) || cp === 0x0670 || cp === 0x0640 || (cp >= 0x06d6 && cp <= 0x06ed)) continue;
    let m = ch;
    if ('أإآٱ'.includes(ch)) m = 'ا';
    else if (ch === 'ى') m = 'ي';
    else if (ch === 'ة') m = 'ه';
    else if (ch === 'ؤ') m = 'و';
    else if (ch === 'ئ') m = 'ي';
    else if (cp >= 0x0660 && cp <= 0x0669) m = String(cp - 0x0660);
    else if (cp >= 0x06f0 && cp <= 0x06f9) m = String(cp - 0x06f0);
    else if (!/[\p{L}\p{N}]/u.test(ch)) {
      pending = true;
      continue;
    }
    if (pending && out) out += ' ';
    pending = false;
    out += m.toLowerCase();
  }
  return out;
}

/** Minimum edit distance between `pat` and any substring of `text` (Sellers' algorithm). */
export function substringDistance(pat: string, text: string): number {
  const p = [...pat];
  const m = p.length;
  let prev = Array.from({ length: m + 1 }, (_, i) => i);
  let cur = new Array<number>(m + 1).fill(0);
  let best = prev[m];
  for (const tc of text) {
    cur[0] = 0;
    for (let i = 1; i <= m; i++) {
      const cost = p[i - 1] === tc ? 0 : 1;
      cur[i] = Math.min(prev[i - 1] + cost, prev[i] + 1, cur[i - 1] + 1);
    }
    best = Math.min(best, cur[m]);
    [prev, cur] = [cur, prev];
  }
  return best;
}

/** `hay` and `q` must already be normalised. Substring first, then a small typo budget. */
export function matches(hay: string, q: string): boolean {
  if (!q) return true;
  if (hay.includes(q)) return true;
  const n = [...q].length;
  if (n < 4) return false;
  const budget = n <= 5 ? 1 : n <= 9 ? 2 : 3;
  return substringDistance(q, hay) <= budget;
}

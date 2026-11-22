// Fails the build when a language is missing a key, or the UI uses a key that doesn't exist.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { createRequire } from 'node:module';

const src = readFileSync('src/lib/i18n.ts', 'utf8');
const grab = (name) => {
  const m = src.match(new RegExp(`const ${name}: Dict = \\{([\\s\\S]*?)\\n\\};`));
  return new Set([...m[1].matchAll(/'([^']+)':/g)].map((x) => x[1]));
};
const en = grab('en'), ar = grab('ar');
let bad = 0;
for (const k of en) if (!ar.has(k)) { console.error('missing in ar:', k); bad++; }
for (const k of ar) if (!en.has(k)) { console.error('missing in en:', k); bad++; }

const files = [];
(function walk(d) { for (const f of readdirSync(d)) { const p = join(d, f); statSync(p).isDirectory() ? walk(p) : /\.(svelte|ts)$/.test(f) && files.push(p); } })('src');
const used = new Set();
const prefixes = new Set();
for (const f of files) {
  const s = readFileSync(f, 'utf8');
  for (const m of s.matchAll(/\bt\('([a-zA-Z0-9_.]+)'(\s*\+)?/g)) (m[2] ? prefixes : used).add(m[1]);
  for (const m of s.matchAll(/key: '(nav\.[a-z]+)'/g)) used.add(m[1]);
  for (const m of s.matchAll(/'(nav\.[a-z]+)'/g)) used.add(m[1]);
}
for (const k of used) if (!en.has(k)) { console.error('unknown key:', k); bad++; }
for (const p of prefixes) if (![...en].some((k) => k.startsWith(p))) { console.error('unknown prefix:', p); bad++; }
if (bad) process.exit(1);
console.log(`i18n ok: ${en.size} keys, en/ar in sync`);

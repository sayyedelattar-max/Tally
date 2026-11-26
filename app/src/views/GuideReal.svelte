<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import { ui, t, go } from '../lib/store.svelte';
  import { api, type Chan, type GuideRow, type Group, type Prog } from '../lib/backend';
  import { artFor } from '../lib/art';

  const PAGE = 40;
  const PX = 4.5; // pixels per minute on the timeline
  const HOURS = 14;

  type Source = 'fav' | 'all' | string; // string = "group:<name>"
  let source = $state<Source>('all');
  let view = $state<'now' | 'line'>((() => { try { return localStorage.getItem('elara.guide') === 'line' ? 'line' : 'now'; } catch { return 'now'; } })());
  let groups = $state<Group[]>([]);
  let rows = $state<GuideRow[]>([]);
  let more = $state(false);
  let loading = $state(true);
  let syncing = $state(false);
  let note = $state('');
  let picked = $state<{ row: GuideRow; p: Prog } | null>(null);
  let scroller: HTMLElement | undefined = $state();
  let tick = $state(Math.floor(Date.now() / 1000));

  const t0 = $derived(Math.floor((tick - 3600) / 1800) * 1800); // timeline starts an hour back, on a half hour
  const t1 = $derived(t0 + HOURS * 3600);
  const hourTicks = $derived(Array.from({ length: HOURS * 2 + 1 }, (_, i) => t0 + i * 1800));

  const hm = (s: number) => { const d = new Date(s * 1000); return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`; };
  const x = (s: number) => Math.max(0, ((Math.min(Math.max(s, t0), t1) - t0) / 60) * PX);
  const initials = (n: string) => n.replace(/[^\p{L}\p{N} ]/gu, '').trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join('').toUpperCase();
  const pctDone = (p: Prog) => Math.min(100, Math.max(0, ((tick - p.start) / (p.stop - p.start)) * 100));
  const nowOf = (r: GuideRow) => r.progs.find((p) => p.start <= tick && p.stop > tick);
  const nextOf = (r: GuideRow) => r.progs.find((p) => p.start >= (nowOf(r)?.stop ?? tick));
  const left = (p: Prog) => { const m = Math.max(1, Math.round((p.stop - tick) / 60)); return m >= 60 ? `${Math.floor(m / 60)}h ${m % 60}m` : `${m}m`; };
  const mins = (p: Prog) => Math.round((p.stop - p.start) / 60);

  async function load(reset: boolean) {
    loading = true;
    note = '';
    try {
      const page = await api.guideRows(source, t0, t1, reset ? 0 : rows.length, PAGE);
      rows = reset ? page : [...rows, ...page];
      more = page.length === PAGE;
      if (reset) picked = null;
    } catch (e) {
      note = String(e);
    }
    loading = false;
  }

  async function sync(force = false) {
    try {
      const st = await api.guideStatus();
      if (!force && st.count > 0 && st.newest > Math.floor(Date.now() / 1000) + 3600) return;
      syncing = true;
      const n = await api.guideSync();
      if (n === 0) note = t('gd.none');
    } catch (e) {
      note = t('gd.failed');
    }
    syncing = false;
  }

  async function start() {
    try {
      groups = await api.groups(false);
      const favs = await api.favorites();
      source = favs.length ? 'fav' : 'all';
    } catch {}
    await load(true);
    loading = true;
    await sync();
    await load(true);
  }

  onMount(() => {
    start();
    const i = setInterval(() => (tick = Math.floor(Date.now() / 1000)), 30_000);
    return () => clearInterval(i);
  });

  function pick(s: Source) {
    source = s;
    load(true);
  }
  function setView(v: 'now' | 'line') {
    view = v;
    try { localStorage.setItem('elara.guide', v); } catch {}
  }
  $effect(() => {
    if (view === 'line' && scroller && rows.length) scroller.scrollLeft = Math.max(0, x(tick) - 90);
  });

  function watch(r: GuideRow) {
    const list: Chan[] = rows.map((g) => g.channel);
    ui.rplay = { list, index: Math.max(0, rows.indexOf(r)) };
  }
  const groupVal = $derived(source.startsWith('group:') ? source : '');
</script>

<div class="head">
  <div class="seg" role="group" aria-label={t('gd.source')}>
    <button class:on={source === 'fav'} onclick={() => pick('fav')}>{t('fav.title')}</button>
    <button class:on={source === 'all'} onclick={() => pick('all')}>{t('common.all')}</button>
  </div>
  <label class="sel">
    <select value={groupVal} onchange={(e) => e.currentTarget.value && pick(e.currentTarget.value)} aria-label={t('real.groups')}>
      <option value="">{t('gd.category')}</option>
      {#each groups as g (g.name)}<option value={'group:' + g.name}>{g.name}</option>{/each}
    </select>
  </label>
  <span class="sp"></span>
  <div class="seg" role="group" aria-label={t('gd.view')}>
    <button class:on={view === 'now'} onclick={() => setView('now')}>{t('gd.now')}</button>
    <button class:on={view === 'line'} onclick={() => setView('line')}>{t('gd.timeline')}</button>
  </div>
  <button class="ico" onclick={async () => { await sync(true); await load(true); }} aria-label={t('gd.refresh')} title={t('gd.refresh')} disabled={syncing}><Icon name="replay" size={18} /></button>
</div>

{#if syncing}<p class="muted info"><i class="spin"></i> {t('gd.updating')}</p>{/if}
{#if note}<p class="muted info">{note}</p>{/if}

{#if !rows.length && !loading && !syncing}
  <div class="empty">
    <h3 class="display">{t('gd.emptyTitle')}</h3>
    <p class="muted">{source === 'fav' ? t('gd.emptyFav') : t('gd.empty')}</p>
    {#if source === 'fav'}<button class="btn" onclick={() => pick('all')}>{t('common.all')}</button>{/if}
    <button class="btn" onclick={() => go('live')}>{t('nav.live')}</button>
  </div>
{:else if view === 'now'}
  <ul class="nowlist">
    {#each rows as r (r.channel.id)}
      {@const a = artFor(r.channel.name)}
      {@const n = nowOf(r)}
      {@const nx = nextOf(r)}
      <li>
        <button class="nrow" onclick={() => watch(r)}>
          <span class="logo" style="background:{a.bg};color:{a.ink}">
            {#if r.channel.logo}<img src={r.channel.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
            <b>{initials(r.channel.name)}</b>
          </span>
          <span class="who">{r.channel.name}</span>
          <span class="cur">
            {#if n}
              <b>{n.title}</b>
              <span class="meta"><span class="ltr">{hm(n.start)}–{hm(n.stop)}</span> · {t('gd.left').replace('{t}', left(n))}</span>
              <i class="prog"><i style="width:{pctDone(n)}%"></i></i>
            {:else}
              <span class="muted">{t('gd.nothing')}</span>
            {/if}
          </span>
          <span class="nxt muted">
            {#if nx}<span class="lbl">{t('hero.next')}</span><span class="ltr">{hm(nx.start)}</span> {nx.title}{/if}
          </span>
        </button>
      </li>
    {/each}
  </ul>
{:else}
  <div class="tl" dir="ltr">
    <div class="tlscroll" bind:this={scroller}>
      <div class="grid" style="width:{HOURS * 60 * PX + 190}px">
        <div class="axis">
          <span class="corner"></span>
          <div class="ticks" style="width:{HOURS * 60 * PX}px">
            {#each hourTicks as s, i}{#if i % 2 === 0}<span class="tk" style="left:{x(s)}px">{hm(s)}</span>{/if}{/each}
          </div>
        </div>
        {#each rows as r (r.channel.id)}
          {@const a = artFor(r.channel.name)}
          <div class="lane">
            <button class="lab" onclick={() => watch(r)} title={r.channel.name}>
              <span class="logo sm" style="background:{a.bg};color:{a.ink}">
                {#if r.channel.logo}<img src={r.channel.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
                <b>{initials(r.channel.name)}</b>
              </span>
              <span class="ln">{r.channel.name}</span>
            </button>
            <div class="track" style="width:{HOURS * 60 * PX}px">
              {#each r.progs as p}
                {@const live = p.start <= tick && p.stop > tick}
                <button class="blk" class:live class:past={p.stop <= tick} class:on={picked?.p === p}
                  style="left:{x(p.start)}px;width:{Math.max(0, x(p.stop) - x(p.start) - 3)}px"
                  onclick={() => (picked = { row: r, p })}>
                  <b>{p.title}</b>
                  <span class="ltr">{hm(p.start)}</span>
                  {#if live}<i class="fill" style="width:{pctDone(p)}%"></i>{/if}
                </button>
              {/each}
            </div>
          </div>
        {/each}
        <i class="nowline" style="left:{190 + x(tick)}px" aria-hidden="true"></i>
      </div>
    </div>
  </div>
{/if}

{#if more && !loading}<div class="moreb"><button class="btn" onclick={() => load(false)}>{t('real.loadMore')}</button></div>{/if}

{#if picked}
  {@const p = picked.p}
  <div class="card" role="dialog" aria-label={p.title}>
    <button class="x" onclick={() => (picked = null)} aria-label={t('common.close')}><Icon name="x" size={18} /></button>
    <span class="muted small">{picked.row.channel.name}</span>
    <h3 class="display">{p.title}</h3>
    <p class="muted small"><span class="ltr">{hm(p.start)}–{hm(p.stop)}</span> · {mins(p)} {t('gd.min')}{#if p.start <= tick && p.stop > tick} · {t('gd.left').replace('{t}', left(p))}{/if}</p>
    {#if p.desc}<p class="desc">{p.desc}</p>{/if}
    {#if p.start <= tick && p.stop > tick}
      <button class="btn primary" onclick={() => watch(picked!.row)}><Icon name="play" filled size={16} /> {t('gd.watch')}</button>
    {:else if p.start > tick}
      <p class="muted small">{t('gd.later')}</p>
      <button class="btn" onclick={() => watch(picked!.row)}>{t('gd.openChannel')}</button>
    {:else}
      <p class="muted small">{t('gd.ended')}</p>
    {/if}
  </div>
{/if}

<style>
  .head { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; margin: 6px 0 16px; }
  .sp { flex: 1; }
  .sel select { min-height: 40px; padding: 0 12px; border-radius: 999px; border: 1px solid var(--line-2); background: var(--surface); max-width: 240px; }
  .ico { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; color: var(--muted); }
  .ico:hover:not(:disabled) { background: var(--surface-2); color: var(--text); }
  .info { margin: 0 0 12px; display: flex; gap: 8px; align-items: center; }
  .spin { width: 14px; height: 14px; border-radius: 50%; border: 2px solid var(--line-2); border-top-color: var(--text); animation: sp 0.8s linear infinite; }
  @keyframes sp { to { transform: rotate(360deg); } }
  .empty { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; padding: 40px 0; }
  .empty h3 { font-size: 28px; }

  .nowlist { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .nrow { width: 100%; display: grid; grid-template-columns: 56px 170px minmax(0, 1.6fr) minmax(0, 1fr); gap: 18px; align-items: center; padding: 12px 16px; border-radius: 16px; background: var(--surface); transition: background 0.15s, transform 0.15s; }
  .nrow:hover, .nrow:focus-visible { background: var(--surface-2); transform: scale(1.005); outline: none; }
  .logo { position: relative; width: 56px; height: 38px; border-radius: 8px; overflow: hidden; display: grid; place-items: center; font-size: 13px; }
  .logo.sm { width: 40px; height: 28px; font-size: 11px; flex: none; }
  .logo img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; background: #fff; padding: 3px; }
  .who { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cur { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .cur b { font-weight: 600; font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { font-size: 13px; color: var(--muted); }
  .prog { display: block; height: 3px; border-radius: 2px; background: var(--line); overflow: hidden; }
  .prog i { display: block; height: 100%; background: var(--text); opacity: 0.8; }
  .nxt { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nxt .lbl { margin-inline-end: 8px; font-size: 11px; letter-spacing: 0.06em; text-transform: uppercase; }

  .tl { border: 1px solid var(--line); border-radius: 18px; overflow: hidden; background: var(--surface); }
  .tlscroll { overflow: auto; max-height: calc(100dvh - 230px); }
  .grid { position: relative; }
  .axis { display: flex; position: sticky; top: 0; z-index: 3; background: var(--surface); border-bottom: 1px solid var(--line); height: 34px; }
  .corner { flex: none; width: 190px; position: sticky; left: 0; background: var(--surface); z-index: 4; }
  .ticks { position: relative; }
  .tk { position: absolute; top: 8px; transform: translateX(-50%); font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .lane { display: flex; border-bottom: 1px solid var(--line); }
  .lab { flex: none; width: 190px; position: sticky; left: 0; z-index: 2; display: flex; gap: 10px; align-items: center; padding: 8px 12px; background: var(--surface); border-inline-end: 1px solid var(--line); }
  .ln { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { position: relative; height: 60px; background-image: repeating-linear-gradient(to right, transparent 0, transparent calc(30 * 4.5px - 1px), var(--line) calc(30 * 4.5px - 1px), var(--line) calc(30 * 4.5px)); }
  .blk { position: absolute; top: 6px; bottom: 6px; padding: 6px 10px; border-radius: 10px; background: var(--surface-3); overflow: hidden; display: flex; flex-direction: column; justify-content: center; gap: 1px; text-align: start; }
  .blk b { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .blk span { font-size: 11px; color: var(--muted); }
  .blk:hover, .blk.on { background: color-mix(in srgb, var(--text) 20%, var(--surface-3)); outline: none; }
  .blk.past { opacity: 0.5; }
  .blk.live { background: color-mix(in srgb, var(--accent) 22%, var(--surface-3)); }
  .fill { position: absolute; left: 0; bottom: 0; height: 3px; background: var(--accent); }
  .nowline { position: absolute; top: 0; bottom: 0; width: 2px; background: var(--live); z-index: 1; pointer-events: none; opacity: 0.85; }
  .moreb { display: grid; place-items: center; margin: 16px 0; }

  .card { position: fixed; inset-inline-end: 28px; bottom: 28px; width: min(380px, calc(100vw - 32px)); display: flex; flex-direction: column; gap: 8px; align-items: flex-start; padding: 20px; border-radius: 20px; background: color-mix(in srgb, var(--surface-2) 92%, transparent); backdrop-filter: blur(20px); border: 1px solid var(--line-2); box-shadow: 0 24px 60px -12px rgb(0 0 0 / 0.7); z-index: 20; }
  .card h3 { font-size: 22px; line-height: 1.15; }
  .card .desc { font-size: 14px; max-height: 120px; overflow-y: auto; line-height: 1.5; }
  .card .x { position: absolute; inset-inline-end: 10px; top: 10px; width: 32px; height: 32px; display: grid; place-items: center; border-radius: 50%; color: var(--muted); }
  .card .x:hover { background: var(--surface-3); color: var(--text); }
  .small { font-size: 13px; }
  @media (max-width: 900px) {
    .nrow { grid-template-columns: 56px minmax(0, 1fr); }
    .who, .nxt { display: none; }
  }
</style>

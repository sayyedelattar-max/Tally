<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import TitleSheet from '../components/TitleSheet.svelte';
  import { t } from '../lib/store.svelte';
  import { api, type Shelf, type TitleItem } from '../lib/backend';

  let { kind }: { kind: 'movie' | 'series' } = $props();

  const PER_ROW = 24;
  const PAGE = 60;

  let shelves = $state<Shelf[]>([]);
  let loading = $state(true);
  let error = $state('');
  let q = $state('');
  let group = $state<string | null>(null); // a category opened as a grid
  let grid = $state<TitleItem[]>([]);
  let more = $state(false);
  let busy = $state(false);
  let open = $state<TitleItem | null>(null);
  let ticket = 0;
  let timer: ReturnType<typeof setTimeout>;

  const searching = $derived(q.trim().length > 0);
  const gridMode = $derived(searching || group !== null);

  async function loadShelves() {
    loading = true;
    error = '';
    try {
      shelves = await api.shelves(kind, PER_ROW, 40);
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  async function loadGrid(reset: boolean) {
    const mine = ++ticket;
    busy = true;
    try {
      const offset = reset ? 0 : grid.length;
      const rows = await api.titles({ kind, group: searching ? null : group, query: searching ? q : null, offset, limit: PAGE });
      if (mine !== ticket) return;
      grid = reset ? rows : [...grid, ...rows];
      more = !searching && rows.length === PAGE;
    } catch (e) {
      if (mine === ticket) error = String(e);
    }
    busy = false;
  }

  function openGroup(name: string | null) {
    group = name;
    grid = [];
    if (name !== null) loadGrid(true);
  }

  function onsearch() {
    clearTimeout(timer);
    timer = setTimeout(() => (searching ? loadGrid(true) : (grid = [])), 250);
  }

  onMount(loadShelves);
  $effect(() => () => clearTimeout(timer));
  const label = (s: Shelf) => (s.recent ? t('vod.recent') : s.name || t('common.all'));
</script>

{#snippet card(it: TitleItem)}
  <button class="card" onclick={() => (open = it)} title={it.name}>
    <span class="art">
      {#if it.poster}<img src={it.poster} alt="" loading="lazy" decoding="async" />{/if}
      <span class="ph display" aria-hidden="true">{it.name.slice(0, 1)}</span>
      {#if it.rating > 0}<span class="rt mono">★ {it.rating.toFixed(1)}</span>{/if}
    </span>
    <span class="nm">{it.name}</span>
  </button>
{/snippet}

<div class="page">
  <div class="tools">
    {#if group !== null && !searching}
      <button class="btn" onclick={() => openGroup(null)}><Icon name="back" size={16} /> {t('vod.back')}</button>
      <h2 class="display">{group || t('common.all')}</h2>
    {/if}
    <label class="find field">
      <Icon name="search" size={18} />
      <input type="search" bind:value={q} oninput={onsearch} placeholder={t(kind === 'movie' ? 'vod.search.movie' : 'vod.search.series')} aria-label={t(kind === 'movie' ? 'vod.search.movie' : 'vod.search.series')} />
    </label>
  </div>

  {#if loading}
    <p class="muted center" role="status"><span class="spin"></span> {t('vod.loading')}</p>
  {:else if error}
    <div class="center">
      <p class="err" role="alert">{t('vod.error')}: {error}</p>
      <button class="btn" onclick={loadShelves}>{t('vod.retry')}</button>
    </div>
  {:else if gridMode}
    {#if grid.length}
      <div class="grid">{#each grid as it (it.kind + it.id)}{@render card(it)}{/each}</div>
      {#if more}<div class="end"><button class="btn" onclick={() => loadGrid(false)} disabled={busy}>{t('real.loadMore')}</button></div>{/if}
    {:else if !busy}
      <p class="muted center">{t('real.noMatch')}</p>
    {/if}
  {:else if shelves.length === 0}
    <div class="center">
      <h2 class="display">{t('vod.empty')}</h2>
      <p class="muted">{t('vod.emptyLead')}</p>
      <button class="btn" onclick={loadShelves}>{t('vod.retry')}</button>
    </div>
  {:else}
    {#each shelves as s (s.recent ? '\u0000recent' : s.name)}
      <section class="shelf">
        <header>
          <h2 class="display">{label(s)}</h2>
          {#if !s.recent && s.count > s.items.length}
            <button class="more" onclick={() => openGroup(s.name)}>{t('vod.seeAll')} · {s.count}</button>
          {/if}
        </header>
        <div class="row">{#each s.items as it (it.kind + it.id)}{@render card(it)}{/each}</div>
      </section>
    {/each}
  {/if}
</div>

{#if open}<TitleSheet item={open} onclose={() => (open = null)} />{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 22px; }
  .page > * { flex: none; }
  .tools { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
  .tools h2 { margin: 0; font-size: 26px; }
  .find { flex: 1; min-width: 220px; max-width: 520px; margin-inline-start: auto; }
  .find input { flex: 1; min-width: 0; background: none; border: 0; color: var(--text); font-size: 16px; height: 100%; }
  .find input:focus-visible { outline: none; }
  .btn { display: inline-flex; align-items: center; gap: 6px; }
  .shelf header { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
  .shelf h2 { margin: 0; font-size: 22px; }
  .more { color: var(--muted); font-size: 13px; padding: 6px 4px; }
  .more:hover { color: var(--text); }
  .row { display: flex; gap: 14px; overflow-x: auto; padding: 4px 4px 14px; scroll-snap-type: x proximity; }
  .row .card { flex: none; width: 150px; scroll-snap-align: start; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 18px 14px; }
  .card { display: flex; flex-direction: column; gap: 8px; text-align: start; border-radius: 10px; min-width: 0; }
  .art { position: relative; display: block; aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; background: var(--surface-2); transition: transform 0.15s; }
  .card:hover .art, .card:focus-visible .art { transform: translateY(-3px); box-shadow: 0 0 0 2px var(--accent); }
  .art img, .ph { position: absolute; inset: 0; width: 100%; height: 100%; }
  .art img { object-fit: cover; z-index: 1; }
  .ph { display: grid; place-items: center; font-size: 44px; color: var(--muted); }
  .rt { position: absolute; z-index: 2; inset-block-end: 6px; inset-inline-start: 6px; padding: 2px 6px; border-radius: 6px; background: rgba(0, 0, 0, 0.7); color: #fff; font-size: 11px; }
  .nm { font-size: 13px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
  .center { display: flex; flex-direction: column; align-items: flex-start; gap: 12px; padding: 36px 0; }
  .center h2 { font-size: 30px; margin: 0; }
  .err { color: var(--live); overflow-wrap: anywhere; margin: 0; }
  .end { display: grid; place-items: center; min-height: 60px; }
  .spin { display: inline-block; width: 16px; height: 16px; vertical-align: -3px; border-radius: 50%; border: 2px solid var(--line-2); border-top-color: var(--accent); animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 3s; } .art { transition: none; } }
</style>

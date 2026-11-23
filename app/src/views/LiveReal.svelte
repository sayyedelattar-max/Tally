<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import { ui, t, toast, go } from '../lib/store.svelte';
  import { api, type Chan, type Group } from '../lib/backend';
  import { artFor } from '../lib/art';

  const PAGE = 100;

  let groups = $state<Group[]>([]);
  let group = $state('');
  let q = $state('');
  let rows = $state<Chan[]>([]);
  let next = $state<[number, number] | null>(null);
  let loading = $state(false);
  let error = $state('');
  let sentinel: HTMLElement | undefined = $state();
  let ticket = 0;

  const total = $derived(groups.reduce((n, g) => n + g.count, 0));

  async function load(reset: boolean) {
    const mine = ++ticket;
    loading = true;
    error = '';
    try {
      const page = await api.channels({ group: group || null, query: q.trim() || null, after: reset ? null : next, limit: PAGE });
      if (mine !== ticket) return;
      rows = reset ? page.rows : [...rows, ...page.rows];
      next = page.next;
    } catch (e) {
      if (mine === ticket) error = String(e);
    } finally {
      if (mine === ticket) loading = false;
    }
  }

  onMount(async () => {
    try {
      groups = await api.groups();
    } catch (e) {
      error = String(e);
    }
    await load(true);
  });

  // reload when the group changes; wait a moment while typing
  let first = true;
  let timer: ReturnType<typeof setTimeout>;
  $effect(() => {
    group;
    q;
    if (first) {
      first = false;
      return;
    }
    clearTimeout(timer);
    timer = setTimeout(() => load(true), 180);
    return () => clearTimeout(timer);
  });

  // load the next page when the end of the list scrolls into view
  $effect(() => {
    if (!sentinel) return;
    const io = new IntersectionObserver((e) => {
      if (e[0].isIntersecting && next && !loading) load(false);
    }, { rootMargin: '600px' });
    io.observe(sentinel);
    return () => io.disconnect();
  });

  async function fav(c: Chan) {
    const v = !c.fav;
    c.fav = v;
    try {
      await api.setFavorite(c.id, v);
    } catch (e) {
      c.fav = !v;
      toast(String(e));
    }
  }

  const initials = (n: string) => n.replace(/[^\p{L}\p{N} ]/gu, '').trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join('').toUpperCase();
</script>

{#if ui.real.loaded && !ui.real.items.length}
  <div class="none">
    <h2 class="display">{t('real.noPlaylist')}</h2>
    <p class="muted">{t('real.noPlaylistLead')}</p>
    <button class="btn primary" onclick={() => go('playlists')}><Icon name="plus" size={18} /> {t('pl.add')}</button>
  </div>
{:else}
  <div class="wrap">
    <aside class="groups" aria-label={t('real.groups')}>
      <button class="g" class:on={group === ''} onclick={() => (group = '')}><span>{t('common.all')}</span><span class="mono muted">{total}</span></button>
      {#each groups as g (g.name)}
        <button class="g" class:on={group === g.name} onclick={() => (group = g.name)}><span class="gn">{g.name || '—'}</span><span class="mono muted">{g.count}</span></button>
      {/each}
    </aside>

    <section class="list">
      <label class="field find">
        <Icon name="search" size={18} />
        <input bind:value={q} placeholder={t('real.searchCh')} aria-label={t('real.searchCh')} autocomplete="off" />
      </label>

      {#if error}<p class="err" role="alert">{error}</p>{/if}

      <ul>
        {#each rows as c (c.id)}
          {@const a = artFor(c.name)}
          <li>
            <button class="row" onclick={() => toast(t('real.playSoon'))}>
              <span class="num mono">{c.number ?? ''}</span>
              <span class="logo" style="background:{a.bg};color:{a.ink}">
                {#if c.logo}<img src={c.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
                <b>{initials(c.name)}</b>
              </span>
              <span class="nm"><b>{c.name}</b><span class="muted">{c.group}</span></span>
            </button>
            <button class="star" class:on={c.fav} aria-pressed={c.fav} aria-label={t('live.favorite')} onclick={() => fav(c)}><Icon name="star" size={18} filled={c.fav} /></button>
          </li>
        {/each}
      </ul>

      {#if !rows.length && !loading && !error}<p class="empty">{t('real.noMatch')}</p>{/if}
      <div bind:this={sentinel} class="end mono muted">{#if loading}…{:else if next}<button class="btn" onclick={() => load(false)}>{t('real.loadMore')}</button>{/if}</div>
    </section>
  </div>
{/if}

<style>
  .none { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; padding: 40px 0; }
  .none h2 { font-size: 32px; }
  .wrap { display: grid; grid-template-columns: 250px minmax(0, 1fr); gap: 24px; align-items: start; }
  .groups { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; max-height: calc(100dvh - 140px); overflow-y: auto; padding-inline-end: 6px; }
  .g { display: flex; justify-content: space-between; gap: 10px; align-items: center; min-height: 40px; padding: 0 12px; border-radius: 8px; text-align: start; }
  .gn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .g:hover { background: var(--surface-2); }
  .g.on { background: var(--surface-3); color: var(--text); box-shadow: inset 3px 0 0 var(--accent); }
  :global([dir='rtl']) .g.on { box-shadow: inset -3px 0 0 var(--accent); }
  .find { margin-bottom: 12px; max-width: 520px; }
  .find input { flex: 1; min-width: 0; background: none; border: 0; color: var(--text); font-size: 16px; height: 100%; }
  .find input:focus-visible { outline: none; }
  .err { color: var(--live); overflow-wrap: anywhere; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  li { display: flex; align-items: center; border-bottom: 1px solid var(--line); }
  .row { flex: 1; min-width: 0; display: flex; align-items: center; gap: 14px; min-height: 60px; padding: 6px 8px; text-align: start; border-radius: 8px; }
  .row:hover { background: var(--surface-2); }
  .num { width: 44px; text-align: end; color: var(--muted); flex: none; }
  .logo { position: relative; flex: none; width: 52px; height: 36px; border-radius: 6px; overflow: hidden; display: grid; place-items: center; font-size: 13px; }
  .logo img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; background: #fff; padding: 3px; }
  .nm { display: flex; flex-direction: column; min-width: 0; }
  .nm b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm .muted { font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .star { flex: none; width: 44px; height: 44px; display: grid; place-items: center; color: var(--muted); border-radius: 8px; }
  .star:hover { background: var(--surface-2); color: var(--text); }
  .star.on { color: var(--accent); }
  .end { min-height: 60px; display: grid; place-items: center; }
  @media (max-width: 900px) {
    .wrap { grid-template-columns: 1fr; }
    .groups { position: static; flex-direction: row; max-height: none; overflow-x: auto; overflow-y: hidden; padding: 0 0 8px; gap: 8px; }
    .g { flex: none; border: 1px solid var(--line-2); border-radius: 999px; gap: 8px; }
    .g.on, :global([dir='rtl']) .g.on { box-shadow: none; border-color: var(--text); }
  }
</style>

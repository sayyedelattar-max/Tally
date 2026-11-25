<script lang="ts">
  import { onMount, tick } from 'svelte';
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
  let soff = $state<number | null>(null); // where the next page starts when the list is sorted by us
  type Mode = 'provider' | 'az' | 'mine';
  let mode = $state<Mode>((() => { try { const v = localStorage.getItem('tally.sort'); return v === 'az' || v === 'mine' ? v : 'provider'; } catch { return 'provider'; } })());
  let showHidden = $state(false);
  let editGroups = $state(false);
  const hasMore = $derived(next !== null || soff !== null);
  let loading = $state(false);
  let error = $state('');
  let sentinel: HTMLElement | undefined = $state();
  let ticket = 0;

  const FAV = '\u0000favorites';
  let favCount = $state(0);
  let dragId = $state<number | null>(null);
  let overId = $state<number | null>(null);
  const inFav = $derived(group === FAV);

  const total = $derived(groups.reduce((n, g) => n + g.count, 0));

  async function load(reset: boolean) {
    const mine = ++ticket;
    loading = true;
    error = '';
    try {
      if (inFav) {
        const all = await api.favorites();
        if (mine !== ticket) return;
        favCount = all.length;
        const needle = q.trim().toLowerCase();
        rows = needle ? all.filter((c) => c.name.toLowerCase().includes(needle)) : all;
        next = null;
        soff = null;
        return;
      }
      if (!q.trim() && (mode !== 'provider' || showHidden)) {
        // our own ordering: a category in "my order" is loaded whole so it can be rearranged
        const limit = mode === 'mine' && group ? 6000 : PAGE;
        const page = await api.channelsSorted({ group: group || null, mode, showHidden, offset: reset ? 0 : rows.length, limit });
        if (mine !== ticket) return;
        rows = reset ? page : [...rows, ...page];
        soff = page.length === limit ? rows.length : null;
        next = null;
        return;
      }
      soff = null;
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
      favCount = (await api.favorites()).length;
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
    mode;
    showHidden;
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
      if (e[0].isIntersecting && hasMore && !loading) load(false);
    }, { rootMargin: '600px' });
    io.observe(sentinel);
    return () => io.disconnect();
  });

  async function fav(c: Chan) {
    const v = !c.fav;
    c.fav = v;
    favCount += v ? 1 : -1;
    try {
      await api.setFavorite(c.id, v);
      if (inFav && !v) rows = rows.filter((x) => x.id !== c.id);
    } catch (e) {
      c.fav = !v;
      favCount += v ? -1 : 1;
      toast(String(e));
    }
  }

  function setMode(m: Mode) {
    mode = m;
    try { localStorage.setItem('tally.sort', m); } catch {}
  }

  async function hide(c: Chan) {
    const v = !c.hidden;
    c.hidden = v;
    try {
      await api.setHidden(c.id, v);
      if (v && !showHidden) {
        rows = rows.filter((x) => x.id !== c.id);
        toast(t('ord.hiddenToast'));
      }
      groups = await api.groups(showHidden || editGroups);
    } catch (e) {
      c.hidden = !v;
      toast(String(e));
    }
  }

  async function reloadGroups() {
    try { groups = await api.groups(showHidden || editGroups); } catch (e) { error = String(e); }
  }
  $effect(() => {
    showHidden;
    editGroups;
    if (!first) reloadGroups();
  });

  async function moveGroup(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= groups.length) return;
    const copy = [...groups];
    [copy[i], copy[j]] = [copy[j], copy[i]];
    groups = copy;
    try { await api.reorderGroups(copy.map((g) => g.name)); } catch (e) { toast(String(e)); reloadGroups(); }
  }

  async function hideGroup(g: Group) {
    const v = !g.hidden;
    g.hidden = v;
    try {
      await api.setGroupHidden(g.name, v);
      if (v && group === g.name) group = '';
      if (v && !(showHidden || editGroups)) groups = groups.filter((x) => x.name !== g.name);
    } catch (e) {
      g.hidden = !v;
      toast(String(e));
    }
  }

  function open(i: number) {
    ui.rplay = { list: [...rows], index: i };
  }

  // ---- the user's own order of favourites
  const canSort = $derived(!q.trim() && (inFav || (mode === 'mine' && group !== '' && soff === null)));

  async function place(from: number, to: number) {
    if (from === to || to < 0 || to >= rows.length) return;
    const moved = rows[from];
    const copy = rows.filter((_, i) => i !== from);
    copy.splice(to, 0, moved);
    rows = copy;
    try {
      await (inFav ? api.reorder(copy.map((c) => c.id)) : api.reorderChannels(copy.map((c) => c.id)));
    } catch (e) {
      toast(String(e));
      load(true);
    }
  }

  async function nudge(i: number, d: number) {
    const id = rows[i].id;
    await place(i, i + d);
    await tick();
    document.querySelector<HTMLElement>(`[data-row="${id}"]`)?.focus();
  }

  function rowKey(e: KeyboardEvent, i: number) {
    if (canSort && e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      e.preventDefault();
      nudge(i, e.key === 'ArrowUp' ? -1 : 1);
    }
  }

  function drop(e: DragEvent, to: number) {
    e.preventDefault();
    const from = rows.findIndex((c) => c.id === dragId);
    dragId = overId = null;
    if (from >= 0) place(from, to);
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
      <button class="g" class:on={inFav} onclick={() => (group = FAV)}><span class="gn"><Icon name="star" size={15} filled /> {t('fav.title')}</span><span class="mono muted">{favCount}</span></button>
      <button class="g" class:on={group === ''} onclick={() => (group = '')}><span>{t('common.all')}</span><span class="mono muted">{total}</span></button>
      {#each groups as g, gi (g.name)}
        <div class="grow" class:dim={g.hidden}>
          <button class="g" class:on={group === g.name} onclick={() => (group = g.name)}><span class="gn">{g.name || '—'}</span><span class="mono muted">{g.count}</span></button>
          {#if editGroups}
            <button class="mv sm" disabled={gi === 0} aria-label={t('fav.up')} onclick={() => moveGroup(gi, -1)}><span class="up"><Icon name="down" size={16} /></span></button>
            <button class="mv sm" disabled={gi === groups.length - 1} aria-label={t('fav.down')} onclick={() => moveGroup(gi, 1)}><Icon name="down" size={16} /></button>
            <button class="mv sm" aria-pressed={g.hidden} aria-label={g.hidden ? t('ord.showGroup') : t('ord.hideGroup')} title={g.hidden ? t('ord.showGroup') : t('ord.hideGroup')} onclick={() => hideGroup(g)}><Icon name="eyeoff" size={16} /></button>
          {/if}
        </div>
      {/each}
      <button class="btn editg" aria-pressed={editGroups} onclick={() => (editGroups = !editGroups)}><Icon name="sliders" size={16} /> {editGroups ? t('ord.doneGroups') : t('ord.editGroups')}</button>
    </aside>

    <section class="list">
      <label class="field find">
        <Icon name="search" size={18} />
        <input bind:value={q} placeholder={t('real.searchCh')} aria-label={t('real.searchCh')} autocomplete="off" />
      </label>

      {#if !inFav}
        <div class="tools">
          <div class="seg" role="group" aria-label={t('ord.sort')}>
            {#each ['provider', 'az', 'mine'] as m}
              <button class:on={mode === m} onclick={() => setMode(m as Mode)}>{t('ord.' + m)}</button>
            {/each}
          </div>
          <button class="btn" aria-pressed={showHidden} class:on={showHidden} onclick={() => (showHidden = !showHidden)}>{t('ord.showHidden')}</button>
        </div>
        {#if mode === 'mine' && group === '' && !q.trim()}<p class="hint muted">{t('ord.pickGroup')}</p>{/if}
      {/if}

      {#if error}<p class="err" role="alert">{error}</p>{/if}

      {#if canSort && rows.length > 1}<p class="hint muted">{t('fav.hint')}</p>{/if}
      <ul>
        {#each rows as c, i (c.id)}
          {@const a = artFor(c.name)}
          <li
            class:dim={c.hidden}
            class:drag={dragId === c.id}
            class:over={overId === c.id && dragId !== c.id}
            draggable={canSort}
            ondragstart={(e) => { dragId = c.id; e.dataTransfer?.setData('text/plain', String(c.id)); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
            ondragover={(e) => { if (canSort && dragId !== null) { e.preventDefault(); overId = c.id; } }}
            ondragend={() => (dragId = overId = null)}
            ondrop={(e) => drop(e, i)}
          >
            {#if canSort}<span class="grip" title={t('fav.drag')} aria-hidden="true"><Icon name="grid" size={18} /></span>{/if}
            <button class="row" data-row={c.id} onclick={() => open(i)} onkeydown={(e) => rowKey(e, i)}>
              <span class="num mono">{canSort ? i + 1 : (c.number ?? '')}</span>
              <span class="logo" style="background:{a.bg};color:{a.ink}">
                {#if c.logo}<img src={c.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
                <b>{initials(c.name)}</b>
              </span>
              <span class="nm"><b>{c.name}</b><span class="muted">{c.group}</span></span>
            </button>
            {#if canSort}
              <button class="mv" disabled={i === 0} aria-label={t('fav.up')} onclick={() => nudge(i, -1)}><span class="up"><Icon name="down" size={18} /></span></button>
              <button class="mv" disabled={i === rows.length - 1} aria-label={t('fav.down')} onclick={() => nudge(i, 1)}><Icon name="down" size={18} /></button>
            {/if}
            {#if !inFav}<button class="mv" aria-pressed={!!c.hidden} aria-label={c.hidden ? t('ord.unhide') : t('ord.hide')} title={c.hidden ? t('ord.unhide') : t('ord.hide')} onclick={() => hide(c)}><Icon name="eyeoff" size={18} /></button>{/if}
            <button class="star" class:on={c.fav} aria-pressed={c.fav} aria-label={t('live.favorite')} onclick={() => fav(c)}><Icon name="star" size={18} filled={c.fav} /></button>
          </li>
        {/each}
      </ul>

      {#if !rows.length && !loading && !error}<p class="empty">{inFav && !q.trim() ? t('fav.empty') : t('real.noMatch')}</p>{/if}
      <div bind:this={sentinel} class="end mono muted">{#if loading}…{:else if hasMore}<button class="btn" onclick={() => load(false)}>{t('real.loadMore')}</button>{/if}</div>
    </section>
  </div>
{/if}

<style>
  .none { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; padding: 40px 0; }
  .none h2 { font-size: 32px; }
  .wrap { display: grid; grid-template-columns: 250px minmax(0, 1fr); gap: 24px; align-items: start; }
  .groups { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; max-height: calc(100dvh - 140px); overflow-y: auto; padding-inline-end: 6px; }
  .g { display: flex; justify-content: space-between; gap: 10px; align-items: center; min-height: 40px; padding: 0 12px; border-radius: 8px; text-align: start; }
  .gn { display: inline-flex; align-items: center; gap: 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
  .hint { margin: 0 0 8px; font-size: 13px; }
  .tools { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
  .grow { display: flex; align-items: center; }
  .grow .g { flex: 1; min-width: 0; }
  .dim { opacity: 0.45; }
  .mv.sm { width: 30px; height: 36px; }
  .editg { margin-top: 10px; align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
  .btn.on { border-color: var(--accent); color: var(--text); }
  li.drag { opacity: 0.4; }
  li.over { box-shadow: 0 -2px 0 var(--accent); }
  li[draggable='true'] { cursor: grab; }
  .grip { display: grid; place-items: center; width: 28px; color: var(--muted); }
  .mv { flex: none; width: 36px; height: 44px; display: grid; place-items: center; color: var(--muted); border-radius: 8px; }
  .mv:hover:not(:disabled) { background: var(--surface-2); color: var(--text); }
  .mv:disabled { opacity: 0.25; }
  .up { display: grid; transform: rotate(180deg); }
  .end { min-height: 60px; display: grid; place-items: center; }
  @media (max-width: 900px) {
    .wrap { grid-template-columns: 1fr; }
    .groups { position: static; flex-direction: row; max-height: none; overflow-x: auto; overflow-y: hidden; padding: 0 0 8px; gap: 8px; }
    .g { flex: none; border: 1px solid var(--line-2); border-radius: 999px; gap: 8px; }
    .g.on, :global([dir='rtl']) .g.on { box-shadow: none; border-color: var(--text); }
  }
</style>

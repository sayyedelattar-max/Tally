<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import { ui, t, toast, go } from '../lib/store.svelte';
  import { api, type Alt, type Chan, type Group } from '../lib/backend';
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
  // ---- select many channels and act on them together
  let selecting = $state(false);
  let sel = $state<number[]>([]);
  let lastIdx = $state<number | null>(null);
  let moveOpen = $state(false);
  let newGroup = $state('');
  const isSel = (id: number) => sel.includes(id);
  function toggleSel(i: number, shift: boolean) {
    const id = rows[i].id;
    if (shift && lastIdx !== null) {
      const [a, b] = [Math.min(lastIdx, i), Math.max(lastIdx, i)];
      const ids = rows.slice(a, b + 1).map((r) => r.id);
      sel = [...new Set([...sel, ...ids])];
    } else {
      sel = isSel(id) ? sel.filter((x) => x !== id) : [...sel, id];
    }
    lastIdx = i;
  }
  function endSelect() { selecting = false; sel = []; moveOpen = false; lastIdx = null; }
  async function bulk(action: 'favorite' | 'unfavorite' | 'hide' | 'unhide' | 'move', g?: string) {
    if (!sel.length) return;
    const n = sel.length;
    try {
      await api.bulk(idsFor(action), action, g);
      toast(t('sel.done').replace('{n}', String(n)));
      sel = [];
      moveOpen = false;
      newGroup = '';
      await reloadGroups();
      await load(true);
    } catch (e) {
      toast(String(e));
    }
  }

  // ---- one entry per channel, with a quality to choose (display only; the provider's list is untouched)
  let merged = $state((() => { try { return localStorage.getItem('elara.merge') === '1'; } catch { return false; } })());
  let qprefs: Record<string, number> = (() => { try { return JSON.parse(localStorage.getItem('elara.q') || '{}'); } catch { return {}; } })();
  let qmenu = $state<number | null>(null);
  const realOf = (c: Chan): Chan => (c.alts ? (c.alts.find((a) => a.chan.id === c.id)?.chan ?? c) : c);
  function asRow(m: { key: string; name: string; variants: { channel: Chan; label: string; score: number; hevc: boolean }[] }): Chan {
    const alts: Alt[] = m.variants.map((v) => ({ chan: v.channel, label: v.label, score: v.score, hevc: v.hevc }));
    const want = qprefs[m.key];
    const pick = alts.find((a) => a.chan.id === want) ?? alts.find((a) => !a.hevc) ?? alts[0];
    return alts.length > 1 ? { ...pick.chan, name: m.name, alts, mkey: m.key } : pick.chan;
  }
  function chooseQ(i: number, a: Alt) {
    const r = rows[i];
    if (!r.alts || !r.mkey) return;
    qprefs[r.mkey] = a.chan.id;
    try { localStorage.setItem('elara.q', JSON.stringify(qprefs)); } catch {}
    rows[i] = { ...a.chan, name: r.name, alts: r.alts, mkey: r.mkey };
    qmenu = null;
  }
  const label = (c: Chan) => c.alts?.find((a) => a.chan.id === c.id)?.label ?? '';
  const idsFor = (action: string) => sel.flatMap((id) => {
    const r = rows.find((x) => x.id === id);
    return r?.alts && (action === 'hide' || action === 'unhide' || action === 'move') ? r.alts.map((a) => a.chan.id) : [id];
  });

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
      if (merged) {
        const list = await api.merged(group || null, q.trim() || null, showHidden);
        if (mine !== ticket) return;
        let out = list.map(asRow);
        if (mode === 'az') out = [...out].sort((a, b) => a.name.localeCompare(b.name));
        rows = out;
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
    merged;
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
      if (c.alts) await api.bulk(c.alts.map((a) => a.chan.id), v ? 'hide' : 'unhide');
      else await api.setHidden(c.id, v);
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

  let gDrag = $state<string | null>(null);
  let gOver = $state<string | null>(null);
  async function dropGroup(to: number) {
    const from = groups.findIndex((g) => g.name === gDrag);
    gDrag = gOver = null;
    if (from < 0 || from === to) return;
    const copy = [...groups];
    const [m] = copy.splice(from, 1);
    copy.splice(to, 0, m);
    groups = copy;
    try { await api.reorderGroups(copy.map((g) => g.name)); } catch (e) { toast(String(e)); reloadGroups(); }
  }

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
    if (selecting) return;
    ui.rplay = { list: rows.map(realOf), index: i };
  }

  // ---- the user's own order of favourites
  const canSort = $derived(!selecting && !merged && !q.trim() && (inFav || (mode === 'mine' && group !== '' && soff === null)));

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
      <div class="ghead"><b>{t('real.groups')}</b><button class="btn editg" class:on={editGroups} aria-pressed={editGroups} onclick={() => (editGroups = !editGroups)}><Icon name="sliders" size={16} /> {editGroups ? t('ord.doneGroups') : t('ord.editGroups')}</button></div>
      {#if editGroups}<p class="hint muted">{t('ord.groupsHint')}</p>{/if}
      <button class="g" class:on={inFav} onclick={() => (group = FAV)}><span class="gn"><Icon name="star" size={15} filled /> {t('fav.title')}</span><span class="mono muted">{favCount}</span></button>
      <button class="g" class:on={group === ''} onclick={() => (group = '')}><span>{t('common.all')}</span><span class="mono muted">{total}</span></button>
      {#each groups as g, gi (g.name)}
        <div class="grow" class:dim={g.hidden} class:gover={gOver === g.name && gDrag !== g.name} class:gdrag={gDrag === g.name}
          draggable={editGroups}
          role="group"
          ondragstart={(e) => { gDrag = g.name; e.dataTransfer?.setData('text/plain', g.name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragover={(e) => { if (editGroups && gDrag !== null) { e.preventDefault(); gOver = g.name; } }}
          ondragend={() => (gDrag = gOver = null)}
          ondrop={(e) => { e.preventDefault(); dropGroup(gi); }}
        >
          {#if editGroups}<span class="grip" aria-hidden="true"><Icon name="grid" size={16} /></span>{/if}
          <button class="g" class:on={group === g.name} onclick={() => (group = g.name)}><span class="gn">{g.name || '—'}</span><span class="mono muted">{g.count}</span></button>
          {#if editGroups}
            <button class="mv sm" disabled={gi === 0} aria-label={t('fav.up')} onclick={() => moveGroup(gi, -1)}><span class="up"><Icon name="down" size={16} /></span></button>
            <button class="mv sm" disabled={gi === groups.length - 1} aria-label={t('fav.down')} onclick={() => moveGroup(gi, 1)}><Icon name="down" size={16} /></button>
            <button class="mv sm" aria-pressed={g.hidden} aria-label={g.hidden ? t('ord.showGroup') : t('ord.hideGroup')} title={g.hidden ? t('ord.showGroup') : t('ord.hideGroup')} onclick={() => hideGroup(g)}><Icon name="eyeoff" size={16} /></button>
          {/if}
        </div>
      {/each}
    </aside>

    <section class="list">
      <label class="field find">
        <Icon name="search" size={18} />
        <input bind:value={q} placeholder={t('real.searchCh')} aria-label={t('real.searchCh')} autocomplete="off" />
      </label>

      <div class="tools">
        {#if !inFav}
          <div class="seg" role="group" aria-label={t('ord.sort')}>
            {#each ['provider', 'az', 'mine'] as m}
              <button class:on={mode === m} onclick={() => setMode(m as Mode)}>{t('ord.' + m)}</button>
            {/each}
          </div>
          <button class="btn" aria-pressed={showHidden} class:on={showHidden} onclick={() => (showHidden = !showHidden)}>{t('ord.showHidden')}</button>
        {/if}
        {#if !inFav}<button class="btn" class:on={merged} aria-pressed={merged} title={t('mrg.tip')} onclick={() => { merged = !merged; try { localStorage.setItem('elara.merge', merged ? '1' : '0'); } catch {} }}><Icon name="layers" size={16} /> {t('mrg.one')}</button>{/if}
        <button class="btn sel" class:on={selecting} aria-pressed={selecting} onclick={() => (selecting ? endSelect() : (selecting = true))}><Icon name="check" size={16} /> {selecting ? t('sel.done2') : t('sel.start')}</button>
      </div>
      {#if !inFav}
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
            {#if selecting}<button class="chk" class:on={isSel(c.id)} role="checkbox" aria-checked={isSel(c.id)} aria-label={c.name} onclick={(e) => toggleSel(i, e.shiftKey)}>{#if isSel(c.id)}<Icon name="check" size={16} />{/if}</button>{/if}
            {#if canSort}<span class="grip" title={t('fav.drag')} aria-hidden="true"><Icon name="grid" size={18} /></span>{/if}
            <button class="row" data-row={c.id} onclick={(e) => (selecting ? toggleSel(i, e.shiftKey) : open(i))} onkeydown={(e) => rowKey(e, i)}>
              <span class="num mono">{i + 1}</span>
              <span class="logo" style="background:{a.bg};color:{a.ink}">
                {#if c.logo}<img src={c.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
                <b>{initials(c.name)}</b>
              </span>
              <span class="nm"><b>{c.name}</b><span class="muted">{c.group}</span></span>
            </button>
            {#if c.alts}
              <span class="qwrap">
                <button class="qchip" aria-haspopup="listbox" aria-expanded={qmenu === c.id} title={t('mrg.pick')} onclick={() => (qmenu = qmenu === c.id ? null : c.id)}>{label(c)} <Icon name="down" size={14} /></button>
                {#if qmenu === c.id}
                  <div class="qpop" role="listbox" aria-label={t('mrg.pick')}>
                    {#each c.alts as a (a.chan.id)}
                      <button role="option" aria-selected={a.chan.id === c.id} class="qopt" class:cur={a.chan.id === c.id} onclick={() => chooseQ(i, a)}>
                        <span>{a.label}</span>
                        {#if a.hevc}<span class="tag">H.265</span>{/if}
                      </button>
                    {/each}
                  </div>
                {/if}
              </span>
            {/if}
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
      {#if selecting}
        <div class="actbar" role="toolbar" aria-label={t('sel.bar')}>
          <b class="cnt">{sel.length ? t('sel.count').replace('{n}', String(sel.length)) : t('sel.hint')}</b>
          <button class="btn" onclick={() => { sel = rows.map((r) => r.id); }}>{t('sel.all')}</button>
          <button class="btn" disabled={!sel.length} onclick={() => (sel = [])}>{t('sel.none')}</button>
          <span class="sp"></span>
          <button class="btn primary" disabled={!sel.length} onclick={() => bulk('favorite')}><Icon name="star" size={16} filled /> {t('sel.fav')}</button>
          {#if inFav}<button class="btn" disabled={!sel.length} onclick={() => bulk('unfavorite')}>{t('sel.unfav')}</button>{/if}
          <div class="mwrap">
            <button class="btn" disabled={!sel.length} aria-expanded={moveOpen} onclick={() => (moveOpen = !moveOpen)}>{t('sel.move')}</button>
            {#if moveOpen}
              <div class="pop" role="dialog" aria-label={t('sel.move')}>
                <div class="newg"><input bind:value={newGroup} placeholder={t('sel.newCat')} aria-label={t('sel.newCat')} onkeydown={(e) => e.key === 'Enter' && newGroup.trim() && bulk('move', newGroup)} /><button class="btn primary" disabled={!newGroup.trim()} onclick={() => bulk('move', newGroup)}>{t('sel.create')}</button></div>
                <button class="opt" onclick={() => bulk('move', '')}>{t('sel.reset')}</button>
                <div class="opts">{#each groups as g (g.name)}<button class="opt" onclick={() => bulk('move', g.name)}><span>{g.name || '—'}</span><span class="mono muted">{g.count}</span></button>{/each}</div>
              </div>
            {/if}
          </div>
          <button class="btn" disabled={!sel.length} onclick={() => bulk(showHidden && rows.some((r) => isSel(r.id) && r.hidden) ? 'unhide' : 'hide')}><Icon name="eyeoff" size={16} /> {t('sel.hide')}</button>
        </div>
      {/if}
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
  .chk { flex: none; width: 26px; height: 26px; margin-inline: 6px; border: 2px solid var(--line-2); border-radius: 50%; display: grid; place-items: center; color: var(--on-accent); }
  .chk.on { background: var(--accent); border-color: var(--accent); }
  .actbar { position: sticky; bottom: 12px; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 12px; padding: 10px 12px; border: 1px solid var(--line-2); border-radius: 16px; background: color-mix(in srgb, var(--surface) 88%, transparent); backdrop-filter: blur(18px); box-shadow: 0 18px 40px -12px rgb(0 0 0 / 0.6); z-index: 4; }
  .actbar .sp { flex: 1; }
  .cnt { font-weight: 600; min-width: 120px; }
  .mwrap { position: relative; }
  .pop { position: absolute; inset-inline-end: 0; bottom: calc(100% + 10px); width: 300px; max-height: 340px; display: flex; flex-direction: column; gap: 6px; padding: 10px; background: var(--surface-2); border: 1px solid var(--line-2); border-radius: 14px; box-shadow: 0 20px 44px -10px rgb(0 0 0 / 0.7); }
  .newg { display: flex; gap: 6px; }
  .newg input { flex: 1; min-width: 0; padding: 0 10px; border: 1px solid var(--line-2); border-radius: 10px; background: var(--bg); min-height: 40px; }
  .opts { overflow-y: auto; display: flex; flex-direction: column; }
  .opt { display: flex; justify-content: space-between; gap: 8px; padding: 9px 10px; border-radius: 8px; text-align: start; }
  .opt:hover { background: var(--surface-3); }
  .qwrap { position: relative; flex: none; }
  .qchip { display: inline-flex; gap: 4px; align-items: center; padding: 6px 12px; border-radius: 999px; background: var(--surface-3); font-size: 12px; font-weight: 600; max-width: 170px; white-space: nowrap; }
  .qchip:hover { background: var(--line-2); }
  .qpop { position: absolute; inset-inline-end: 0; top: calc(100% + 6px); min-width: 210px; max-height: 320px; overflow-y: auto; padding: 6px; display: flex; flex-direction: column; background: var(--surface-2); border: 1px solid var(--line-2); border-radius: 14px; box-shadow: 0 20px 44px -10px rgb(0 0 0 / 0.7); z-index: 6; }
  .qopt { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 9px 12px; border-radius: 8px; font-size: 13px; }
  .qopt:hover { background: var(--surface-3); }
  .qopt.cur { background: var(--surface-3); font-weight: 600; }
  .tag { font-size: 10px; padding: 1px 6px; border-radius: 4px; border: 1px solid var(--line-2); color: var(--muted); }
  .ghead { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 8px; position: sticky; top: 0; background: var(--bg); padding-block: 4px; z-index: 1; }
  .gover { box-shadow: 0 -2px 0 var(--accent); }
  .gdrag { opacity: 0.4; }
  .grow[draggable='true'] { cursor: grab; }
  .editg { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
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

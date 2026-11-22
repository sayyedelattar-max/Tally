<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, t, tune, go, play, openTitle, closeSearch, chName, progTitle } from '../lib/store.svelte';
  import { normalize, matches } from '../lib/norm';
  import { allTitles, channels, clock } from '../lib/mock';

  let q = $state('');
  let input: HTMLInputElement;
  $effect(() => input?.focus());

  const nq = $derived(normalize(q));

  type Hit = { kind: string; title: string; sub: string; run: () => void };

  const hits = $derived.by((): { group: string; items: Hit[] }[] => {
    if (!nq) return [];
    const live: Hit[] = channels
      .filter((c) => !ui.hidden.live.includes(c.cat) && matches(normalize(`${c.name} ${c.nameAr ?? ''} ${c.num}`), nq))
      .slice(0, 5)
      .map((c) => ({ kind: 'live', title: chName(c), sub: `CH ${c.num}`, run: () => { closeSearch(); play({ kind: 'live', id: c.id }); } }));

    const vod: Hit[] = allTitles
      .filter((x) => !ui.hidden[x.kind === 'movie' ? 'movies' : 'series'].includes(x.cat) && matches(normalize(x.title), nq))
      .slice(0, 6)
      .map((x) => ({ kind: x.kind, title: x.title, sub: `${x.year} · ${x.cat}`, run: () => { closeSearch(); openTitle(x); } }));

    const seen = new Set<string>();
    const progs: Hit[] = [];
    for (const c of channels) {
      if (ui.hidden.live.includes(c.cat)) continue;
      for (const p of c.progs) {
        if (p.end <= 0 || p.start > 1440) continue;
        const key = `${p.title}|${c.id}`;
        if (seen.has(key) || !matches(normalize(`${p.title} ${p.titleAr ?? ''}`), nq)) continue;
        seen.add(key);
        progs.push({
          kind: 'prog', title: progTitle(p), sub: `${chName(c)} · ${clock(p.start)}`,
          run: () => { closeSearch(); tune(c.id); go('live'); },
        });
      }
    }
    return [
      { group: t('search.live'), items: live },
      { group: t('search.movies'), items: vod.filter((h) => h.kind === 'movie') },
      { group: t('search.series'), items: vod.filter((h) => h.kind === 'series') },
      { group: t('search.programmes'), items: progs.slice(0, 5) },
    ].filter((g) => g.items.length);
  });

  const total = $derived(hits.reduce((n, g) => n + g.items.length, 0));
  const tries = ['Meridian', 'Salt Road', 'المرفأ', 'harbr'];

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeSearch();
    } else if (e.key === 'Enter' && hits.length && document.activeElement === input) {
      e.preventDefault();
      hits[0].items[0].run();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="scrim" data-overlay>
  <button class="bg" aria-label={t('common.close')} onclick={closeSearch} tabindex="-1"></button>
  <div class="box" role="dialog" aria-label={t('search.placeholder')}>
    <label class="in">
      <Icon name="search" size={22} />
      <input bind:this={input} bind:value={q} placeholder={t('search.placeholder')} aria-label={t('search.placeholder')} autocomplete="off" />
      <kbd class="mono ltr">Esc</kbd>
    </label>
    <div class="res">
      {#if !nq}
        <p class="muted hint">{t('search.try')}</p>
        <div class="chips tries">
          {#each tries as s}<button class="chip" onclick={() => { q = s; input.focus(); }}>{s}</button>{/each}
        </div>
      {:else if !total}
        <p class="empty">{t('search.none')}</p>
      {:else}
        {#each hits as g}
          <div class="g">
            <h3 class="mono muted">{g.group}</h3>
            {#each g.items as h}
              <button class="hit" onclick={h.run}>
                <b>{h.title}</b>
                <span class="muted">{h.sub}</span>
              </button>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
    <p class="foot mono muted">{t('search.foot')}</p>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 9vh 16px 16px;
    background: rgba(5, 6, 7, 0.78);
  }
  .bg {
    position: absolute;
    inset: 0;
    cursor: default;
  }
  .box {
    position: relative;
    width: min(720px, 100%);
    max-height: 80dvh;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--line-2);
    border-radius: 16px;
    overflow: hidden;
  }
  .in {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 20px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  .in input {
    flex: 1;
    min-width: 0;
    height: 64px;
    background: none;
    border: 0;
    color: var(--text);
    font-size: 20px;
  }
  .in input:focus-visible {
    outline: none;
  }
  kbd {
    padding: 2px 6px;
    border: 1px solid var(--line-2);
    border-radius: 4px;
    font-size: 11px;
  }
  .res {
    overflow-y: auto;
    padding: 8px;
    min-height: 120px;
  }
  .hint {
    padding: 14px 12px 4px;
  }
  .tries {
    padding-inline: 8px;
    flex-wrap: wrap;
  }
  .g h3 {
    margin: 12px 12px 4px;
    font-weight: 400;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 11px;
  }
  .hit {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    min-height: 46px;
    padding: 8px 12px;
    border-radius: 8px;
  }
  .hit:hover {
    background: var(--surface-3);
  }
  .hit b {
    font-weight: 600;
  }
  .hit span {
    font-size: 13px;
    white-space: nowrap;
  }
  .foot {
    padding: 10px 20px;
    border-top: 1px solid var(--line);
    font-size: 11px;
  }
</style>

<script lang="ts">
  import Art from '../components/Art.svelte';
  import Icon from '../components/Icon.svelte';
  import { ui, t, play, toast } from '../lib/store.svelte';
  import { artFor } from '../lib/art';
  import { fmtDuration } from '../lib/mock';
  import type { Title } from '../lib/types';

  let { item }: { item: Title } = $props();

  let season = $state(0);
  let listed = $state(false);

  const spec = $derived(artFor(item.title));
  const eps = $derived(item.seasons?.[season] ?? []);
  const next = $derived.by(() => {
    if (!item.seasons) return null;
    for (let s = 0; s < item.seasons.length; s++)
      for (let e = 0; e < item.seasons[s].length; e++) if ((item.seasons[s][e].progress ?? 0) < 1) return { s, e };
    return { s: 0, e: 0 };
  });
  const started = $derived(item.kind === 'movie' ? item.progress !== undefined : !(next?.s === 0 && next?.e === 0 && item.seasons?.[0][0].progress === undefined));

  function playMain() {
    if (item.kind === 'movie') play({ kind: 'vod', id: item.id });
    else play({ kind: 'vod', id: item.id, season: next!.s, ep: next!.e });
  }
  function toggleList() {
    listed = !listed;
    toast(listed ? t('toast.listAdded') : t('toast.listRemoved'));
  }
</script>

<div class="detail" style="--bg-c:{spec.bg};--ink:{spec.ink}">
  <button class="back" onclick={() => (ui.detail = null)}><Icon name="back" size={20} flip /> {t('common.back')}</button>
  <div class="head">
    <div class="poster"><Art seed={item.title} kind="poster" label={item.title} /></div>
    <div class="info">
      <span class="tag">{item.kind === 'movie' ? t('hero.movie') : t('hero.series')}</span>
      <h2 class="display">{item.title}</h2>
      <p class="meta mono">
        {item.year} · {item.cat} ·
        {#if item.kind === 'movie'}{fmtDuration(item.mins ?? 0, ui.lang)}{:else}{item.seasons?.length} {t('detail.seasons')}{/if}
        · <span class="pill">{item.rating}</span>
      </p>
      <p class="syn">{item.synopsis}</p>
      {#if item.progress !== undefined && item.kind === 'movie'}<div class="bar prog"><i style="width:{item.progress * 100}%"></i></div>{/if}
      <div class="actions">
        <button class="hbtn solid" onclick={playMain}>
          <Icon name="play" filled size={18} />
          {#if item.kind === 'movie'}{started ? t('detail.resume') : t('detail.play')}{:else}{started ? t('detail.continue') : t('detail.play')} · S{(next?.s ?? 0) + 1} E{(next?.e ?? 0) + 1}{/if}
        </button>
        <button class="hbtn" onclick={toggleList}>
          <Icon name={listed ? 'check' : 'plus'} size={18} /> {t('detail.myList')}
        </button>
      </div>
    </div>
  </div>

  {#if item.seasons}
    <div class="eps">
      <div class="seg" role="group" aria-label={t('detail.seasons')}>
        {#each item.seasons as _, i}
          <button class:on={season === i} onclick={() => (season = i)}>{t('detail.season')} {i + 1}</button>
        {/each}
      </div>
      <ul>
        {#each eps as ep, i}
          <li>
            <button class="ep" onclick={() => play({ kind: 'vod', id: item.id, season, ep: i })}>
              <span class="mono n">{ep.n}</span>
              <span class="tt">
                <b>{ep.title}</b>
                {#if ep.progress !== undefined && ep.progress < 1}<span class="bar"><i style="width:{ep.progress * 100}%"></i></span>{/if}
              </span>
              <span class="mono muted">{ep.mins} {t('common.min')}</span>
              {#if ep.progress === 1}<Icon name="check" size={18} />{:else}<Icon name="play" size={18} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .detail {
    margin-top: 8px;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    padding: 0 12px 0 8px;
    margin-bottom: 8px;
    color: var(--muted);
    border-radius: var(--radius);
  }
  .back:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .head {
    display: flex;
    gap: 36px;
    padding: 36px 40px;
    border-radius: 18px;
    background: var(--bg-c);
    color: var(--ink);
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .poster {
    width: 220px;
    flex: none;
    border-radius: 10px;
    overflow: hidden;
    outline: 1.5px solid color-mix(in srgb, var(--ink) 30%, transparent);
  }
  .info {
    flex: 1 1 340px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    align-items: flex-start;
    min-width: 0;
  }
  h2 {
    font-size: clamp(44px, 5vw, 72px);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 13px;
    opacity: 0.9;
  }
  .syn {
    max-width: 58ch;
    font-size: 17px;
  }
  .prog {
    width: min(360px, 100%);
    color: var(--ink);
  }
  .actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .hbtn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 46px;
    padding: 0 22px;
    border: 1.5px solid var(--ink);
    border-radius: var(--radius);
    font-weight: 600;
  }
  .hbtn.solid {
    background: var(--ink);
    color: var(--bg-c);
  }
  .hbtn:focus-visible {
    outline-color: var(--ink);
  }

  .eps {
    margin-top: 28px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    align-items: flex-start;
  }
  .eps ul {
    list-style: none;
    margin: 0;
    padding: 0;
    width: 100%;
    max-width: 860px;
  }
  .ep {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) auto 24px;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 60px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--line);
  }
  .ep:hover {
    background: var(--surface);
  }
  .n {
    color: var(--muted);
    font-size: 14px;
  }
  .tt {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .tt .bar {
    width: 140px;
    color: var(--accent);
  }
</style>

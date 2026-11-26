<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { ui, t } from '../lib/store.svelte';
  import { api, type Chan, type EpisodeItem, type SeriesInfo, type TitleItem } from '../lib/backend';

  let { item, onclose }: { item: TitleItem; onclose: () => void } = $props();

  let info = $state<SeriesInfo | null>(null);
  let season = $state(1);
  let loading = $state(false);
  let error = $state('');
  let panel: HTMLElement | undefined = $state();

  const seasons = $derived(info ? [...new Set(info.episodes.map((e) => e.season))] : []);
  const eps = $derived(info ? info.episodes.filter((e) => e.season === season) : []);
  const plot = $derived(item.plot || info?.plot || '');

  async function load() {
    if (item.kind !== 'series') return;
    loading = true;
    error = '';
    try {
      info = await api.series(item.id);
      season = info.episodes[0]?.season ?? 1;
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  const chan = (name: string, group: string, vod: Chan['vod']): Chan => ({ id: 0, tvgId: '', name, group, logo: '', number: null, fav: false, vod });
  const pad = (n: number) => String(n).padStart(2, '0');

  function playMovie() {
    api.recordWatch('movie', item.id).catch(() => {});
    ui.rplay = { list: [chan(item.name, item.year, { kind: 'movie', id: item.id, ext: item.ext })], index: 0 };
  }
  function playEpisode(e: EpisodeItem) {
    if (!info) return;
    api.recordWatch('series', item.id).catch(() => {});
    const list = info.episodes.map((x) => chan(`${item.name} · S${pad(x.season)}E${pad(x.number)}${x.title ? ' · ' + x.title : ''}`, item.name, { kind: 'episode', id: x.id, ext: x.ext }));
    ui.rplay = { list, index: info.episodes.indexOf(e) };
  }

  onMount(() => {
    load();
    panel?.focus();
  });
  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && !ui.rplay) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="scrim" role="presentation" onclick={onclose} data-overlay>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={item.name} tabindex="-1" bind:this={panel} onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <button class="x" onclick={onclose} aria-label={t('common.close')}><Icon name="x" size={20} /></button>
    <div class="top">
      <div class="poster">
        {#if item.poster}<img src={item.poster} alt="" />{:else}<span class="display">{item.name.slice(0, 1)}</span>{/if}
      </div>
      <div class="meta">
        <h2 class="display">{item.name}</h2>
        <p class="mono muted facts">
          {#if item.year}<span>{item.year}</span>{/if}
          {#if item.rating > 0}<span>★ {item.rating.toFixed(1)}</span>{/if}
          {#if item.group}<span>{item.group}</span>{/if}
        </p>
        <p class="plot">{plot || t('vod.noPlot')}</p>
        {#if item.kind === 'movie'}
          <button class="btn primary big" onclick={playMovie}><Icon name="play" size={18} filled /> {t('vod.play')}</button>
        {/if}
      </div>
    </div>

    {#if item.kind === 'series'}
      {#if loading}<p class="muted" role="status">{t('vod.loading')}</p>
      {:else if error}
        <p class="err" role="alert">{t('vod.error')}: {error}</p>
        <button class="btn" onclick={load}>{t('vod.retry')}</button>
      {:else if info}
        {#if seasons.length > 1}
          <div class="seasons" role="tablist" aria-label={t('vod.season')}>
            {#each seasons as s}
              <button class="chip" class:on={s === season} role="tab" aria-selected={s === season} onclick={() => (season = s)}>{t('vod.season')} {s}</button>
            {/each}
          </div>
        {/if}
        <ol class="eps">
          {#each eps as e (e.id)}
            <li>
              <button class="ep" onclick={() => playEpisode(e)}>
                <span class="n mono">{e.number}</span>
                <span class="tx"><b>{e.title || `${t('vod.episode')} ${e.number}`}</b>{#if e.duration}<span class="muted mono">{e.duration}</span>{/if}</span>
                <Icon name="play" size={18} filled />
              </button>
            </li>
          {/each}
        </ol>
        {#if !eps.length}<p class="muted">{t('vod.empty')}</p>{/if}
      {/if}
    {/if}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 50; background: rgba(0, 0, 0, 0.6); display: grid; place-items: center; padding: 16px; }
  .sheet { position: relative; width: min(880px, 100%); max-height: calc(100dvh - 32px); overflow-y: auto; background: var(--surface); border: 1px solid var(--line-2); border-radius: 16px; padding: 24px; display: flex; flex-direction: column; gap: 18px; }
  .sheet:focus-visible { outline: none; }
  .x { position: absolute; inset-block-start: 12px; inset-inline-end: 12px; width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; color: var(--muted); }
  .x:hover { background: var(--surface-2); color: var(--text); }
  .top { display: flex; gap: 22px; align-items: flex-start; }
  .poster { flex: none; width: 190px; aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; background: var(--surface-2); display: grid; place-items: center; font-size: 48px; }
  .poster img { width: 100%; height: 100%; object-fit: cover; }
  .meta { min-width: 0; display: flex; flex-direction: column; gap: 10px; padding-inline-end: 36px; }
  h2 { margin: 0; font-size: 30px; line-height: 1.15; overflow-wrap: anywhere; }
  .facts { display: flex; gap: 14px; flex-wrap: wrap; margin: 0; font-size: 12px; }
  .plot { margin: 0; line-height: 1.6; color: var(--text-2, var(--text)); max-width: 62ch; }
  .big { align-self: flex-start; height: 48px; padding-inline: 22px; display: inline-flex; align-items: center; gap: 8px; }
  .seasons { display: flex; gap: 8px; flex-wrap: wrap; }
  .eps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  .eps li { border-bottom: 1px solid var(--line); }
  .ep { width: 100%; display: flex; align-items: center; gap: 14px; min-height: 56px; padding: 6px 8px; text-align: start; border-radius: 8px; }
  .ep:hover { background: var(--surface-2); }
  .n { width: 32px; text-align: end; color: var(--muted); flex: none; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .err { color: var(--live); overflow-wrap: anywhere; margin: 0; }
  @media (max-width: 640px) {
    .top { flex-direction: column; }
    .poster { width: 140px; }
    .meta { padding-inline-end: 0; }
  }
</style>

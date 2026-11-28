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
    {#if item.poster}<div class="bg" style="background-image:url({JSON.stringify(item.poster).slice(1, -1)})" aria-hidden="true"></div>{/if}
    <button class="x" onclick={onclose} aria-label={t('common.close')}><Icon name="x" size={20} /></button>
    <div class="top">
      <div class="poster">
        {#if item.poster}<img src={item.poster} alt="" />{:else}<span class="display">{item.name.slice(0, 1)}</span>{/if}
      </div>
      <div class="meta">
        <h2 class="display">{item.name}</h2>
        <p class="facts">
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
  .scrim { position: fixed; inset: 0; z-index: 50; background: rgba(0, 0, 0, 0.62); backdrop-filter: blur(14px); display: grid; place-items: center; padding: 16px; }
  .sheet { position: relative; isolation: isolate; width: min(900px, 100%); max-height: calc(100dvh - 32px); overflow-y: auto; background: var(--surface); border: 1px solid var(--line); border-radius: 26px; padding: 34px; display: flex; flex-direction: column; gap: 26px; box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55); }
  .sheet:focus-visible { outline: none; }
  /* the poster, blurred and darkened, as a soft backdrop (Apple TV style) */
  .bg { position: absolute; inset: 0 0 auto 0; height: 360px; z-index: -1; background-size: cover; background-position: center 20%; filter: blur(46px) saturate(1.2); opacity: 0.34; transform: scale(1.25); -webkit-mask-image: linear-gradient(#000 30%, transparent); mask-image: linear-gradient(#000 30%, transparent); }
  .x { position: absolute; inset-block-start: 16px; inset-inline-end: 16px; width: 36px; height: 36px; display: grid; place-items: center; border-radius: 50%; color: var(--muted); background: color-mix(in srgb, var(--text) 10%, transparent); }
  .x:hover { background: color-mix(in srgb, var(--text) 18%, transparent); color: var(--text); }
  .top { display: flex; gap: 28px; align-items: flex-end; }
  .poster { flex: none; width: 200px; aspect-ratio: 2 / 3; border-radius: 14px; overflow: hidden; background: var(--surface-2); display: grid; place-items: center; font-size: 52px; box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5); }
  .poster img { width: 100%; height: 100%; object-fit: cover; }
  .meta { min-width: 0; display: flex; flex-direction: column; gap: 12px; padding-inline-end: 40px; padding-block-end: 6px; }
  h2 { margin: 0; font-size: 36px; font-weight: 700; letter-spacing: -0.02em; line-height: 1.1; overflow-wrap: anywhere; text-wrap: balance; }
  .facts { display: flex; gap: 0; flex-wrap: wrap; margin: 0; font-size: 14px; color: var(--muted); }
  .facts span + span::before { content: '·'; margin-inline: 9px; opacity: 0.7; }
  .plot { margin: 0; line-height: 1.55; font-size: 15px; color: var(--text-2, var(--text)); max-width: 62ch; display: -webkit-box; -webkit-line-clamp: 5; line-clamp: 5; -webkit-box-orient: vertical; overflow: hidden; }
  .big { align-self: flex-start; height: 50px; padding-inline: 28px; display: inline-flex; align-items: center; gap: 9px; margin-block-start: 6px; border-radius: 999px; border: 0; font-size: 16px; font-weight: 650; background: var(--text); color: var(--bg); }
  .big:hover { opacity: 0.9; }
  .seasons { display: flex; gap: 6px; flex-wrap: wrap; padding: 4px; border-radius: 999px; background: var(--surface-2); align-self: flex-start; }
  .seasons .chip { border: 0; border-radius: 999px; background: transparent; padding: 7px 16px; color: var(--muted); font-weight: 600; }
  .seasons .chip.on { background: var(--text); color: var(--bg); }
  .eps { list-style: none; margin: 0; padding: 4px 6px; display: flex; flex-direction: column; border-radius: 18px; background: var(--surface-2); }
  .eps li + li { border-top: 1px solid var(--line); }
  .ep { width: 100%; display: flex; align-items: center; gap: 16px; min-height: 60px; padding: 8px 12px; text-align: start; border-radius: 14px; }
  .ep:hover { background: color-mix(in srgb, var(--text) 8%, transparent); }
  .n { width: 30px; text-align: center; color: var(--muted); flex: none; font-size: 15px; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .tx b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .err { color: var(--live); overflow-wrap: anywhere; margin: 0; }
  @media (max-width: 640px) {
    .sheet { padding: 22px; }
    .top { flex-direction: column; align-items: flex-start; }
    .poster { width: 140px; }
    .meta { padding-inline-end: 0; }
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import TitleSheet from '../components/TitleSheet.svelte';
  import { ui, t, go } from '../lib/store.svelte';
  import { api, expiry, type Chan, type HomeData, type PlaylistInfo, type TitleItem } from '../lib/backend';
  import { artFor } from '../lib/art';
  import { asRow, bestTag } from '../lib/quality';
  import { view } from '../lib/view.svelte';

  let data = $state<HomeData | null>(null);
  let info = $state<PlaylistInfo | null>(null);
  let open = $state<TitleItem | null>(null);
  let err = $state('');

  const hour = new Date().getHours();
  const part = hour < 5 ? 'night' : hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';

  async function load() {
    try {
      const [d, r] = await Promise.all([api.home(), api.playlists()]);
      data = d;
      info = r.items.find((x) => x.id === r.active) ?? null;
    } catch (e) {
      err = String(e);
    }
  }
  onMount(load);
  $effect(() => { if (!ui.rplay && data) load(); });

  const exp = $derived(info ? expiry(info.expiresAt) : null);
  const initials = (n: string) => n.replace(/[^\p{L}\p{N} ]/gu, '').trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join('').toUpperCase();

  function playLive(list: Chan[], i: number) {
    ui.rplay = { list: [...list], index: i };
  }
  // one entry per channel (qualities folded) unless the user turned merging off
  const favs = $derived<Chan[]>(!data ? [] : view.lines && view.channels ? data.favoritesMerged.map(asRow) : data.favorites);
  const recents = $derived<Chan[]>(!data ? [] : view.lines && view.channels ? data.recentMerged.map(asRow) : data.recentLive);
  const hero = $derived(data?.lastLive ?? favs[0] ?? null);
  const heroList = $derived(data?.lastLive ? [data.lastLive, ...recents] : favs);

  const noTitles = $derived(!!data && data.movieCount + data.seriesCount === 0);
</script>

<p class="greet">{t('home.greet.' + part)}</p>

{#if err}<p class="muted">{err}</p>{/if}

{#if hero}
  {@const a = artFor(hero.name)}
  <section class="hero" style="--bg-c:{a.bg};--ink:{a.ink}" aria-label={t('hm.resume')}>
    <div class="hlogo">
      {#if hero.logo}<img src={hero.logo} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
      <b class="display">{initials(hero.name)}</b>
    </div>
    <div class="hcopy">
      <span class="tagline"><i class="dot"></i> {t('hm.resume')}</span>
      <h3 class="display">{hero.name}</h3>
      {#if data?.lastLive && data.nowTitle}
        <p class="np"><b>{data.nowTitle}</b></p>
        <i class="hprog"><i style="width:{Math.min(100, Math.max(0, ((Date.now() / 1000 - data.nowStart) / Math.max(1, data.nowStop - data.nowStart)) * 100))}%"></i></i>
        {#if data.nextTitle}<p class="muted">{t('hero.next')}: {data.nextTitle}</p>{/if}
      {:else}<p class="muted">{hero.group}</p>{/if}
      <button class="btn primary big" onclick={() => playLive(heroList, 0)}><Icon name="play" filled size={20} /> {t('hm.play')}</button>
    </div>
  </section>
{:else if data}
  <section class="hero empty"><p class="muted">{t('hm.noLast')}</p><button class="btn primary" onclick={() => go('live')}>{t('nav.live')}</button></section>
{/if}

{#snippet chans(title: string, list: Chan[])}
  <section>
    <div class="row-head"><h2>{title}</h2></div>
    <div class="scroller">
      {#each list as c, i (c.id)}
        {@const a = artFor(c.name)}
        <button class="ch" onclick={() => playLive(list, i)} title={c.name}>
          <span class="cl" style="background:{a.bg};color:{a.ink}">
            {#if c.logo}<img src={c.logo} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
            <b>{initials(c.name)}</b>
          </span>
          <span class="cn">{c.name}{#if c.alts}{@const bt = bestTag(c)}{#if bt}<i class="qb">{bt}</i>{/if}{/if}</span>
        </button>
      {/each}
    </div>
  </section>
{/snippet}

{#snippet posters(title: string, list: TitleItem[], more: 'movies' | 'series' | null)}
  <section>
    <div class="row-head"><h2>{title}</h2>{#if more}<button class="more muted" onclick={() => go(more)}>{t('common.seeAll')} <Icon name="chevron" size={16} flip /></button>{/if}</div>
    <div class="scroller">
      {#each list as it (it.kind + it.id)}
        <button class="pst" onclick={() => (open = it)} title={it.name}>
          <span class="pimg">{#if it.poster}<img src={it.poster} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}<b class="display">{it.name.slice(0, 1)}</b></span>
          <span class="pn">{it.name}</span>
        </button>
      {/each}
    </div>
  </section>
{/snippet}

{#if data}
  {#if data.continueTitles.length}{@render posters(t('hm.continue'), data.continueTitles, null)}{/if}
  {#if favs.length}{@render chans(t('hm.favs'), favs)}
  {:else}<p class="muted note">{t('hm.noFavs')}</p>{/if}
  {#if recents.length}{@render chans(t('hm.recent'), recents)}{/if}
  {#if data.newMovies.length}{@render posters(t('hm.newMovies'), data.newMovies, 'movies')}{/if}
  {#if data.newSeries.length}{@render posters(t('hm.newSeries'), data.newSeries, 'series')}{/if}
  {#if noTitles}<p class="muted note">{t('hm.empty')}</p>{/if}
{/if}

{#if open}<TitleSheet item={open} onclose={() => (open = null)} />{/if}

<style>
  .greet { margin: 4px 0 14px; color: var(--muted); font-size: 15px; font-weight: 500; }
  .hero { display: flex; gap: 40px; align-items: center; min-height: min(46vh, 420px); padding: 40px 48px; border-radius: 28px; box-shadow: 0 30px 60px -30px rgb(0 0 0 / 0.6); background: linear-gradient(120deg, var(--bg-c), color-mix(in srgb, var(--bg-c) 40%, var(--surface))); color: var(--ink); overflow: hidden; }
  .hero.empty { background: var(--surface); color: var(--text); flex-direction: column; align-items: flex-start; }
  .hlogo { position: relative; flex: none; width: clamp(110px, 18vw, 200px); aspect-ratio: 16 / 10; border-radius: 14px; overflow: hidden; display: grid; place-items: center; background: rgb(255 255 255 / 0.12); font-size: 44px; }
  .hlogo img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; background: #fff; padding: 10px; }
  .hcopy { min-width: 0; display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  .hcopy h3 { font-size: clamp(30px, 5vw, 60px); line-height: 1.02; }
  .hcopy .muted { color: inherit; opacity: 0.75; margin: 0; }
  .tagline { font-size: 13px; font-weight: 600; letter-spacing: 0.04em; display: inline-flex; gap: 8px; align-items: center; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; display: inline-block; }
  .np { font-size: 18px; margin: 0; }
  .hprog { display: block; width: min(320px, 100%); height: 3px; border-radius: 2px; background: rgb(255 255 255 / 0.2); overflow: hidden; }
  .hprog i { display: block; height: 100%; background: currentColor; }
  .big { padding: 14px 28px; border-radius: 999px; font-size: 17px; margin-top: 8px; display: inline-flex; gap: 8px; align-items: center; }
  .more { display: inline-flex; align-items: center; gap: 4px; font-size: 14px; padding: 6px 10px; border-radius: 8px; }
  .more:hover { color: var(--text); background: var(--surface-2); }
  .ch, .pst { transition: transform 0.18s ease, box-shadow 0.18s ease; }
  .ch:hover, .ch:focus-visible, .pst:hover, .pst:focus-visible { transform: scale(1.06); box-shadow: 0 18px 36px -12px rgb(0 0 0 / 0.65); outline: none; z-index: 2; }
  .ch { width: 132px; display: flex; flex-direction: column; gap: 8px; text-align: start; border-radius: 12px; padding: 6px; }
    .cl { position: relative; width: 100%; aspect-ratio: 16 / 10; border-radius: 10px; overflow: hidden; display: grid; place-items: center; font-size: 22px; }
  .cl img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; background: #fff; padding: 8px; }
  .qb { font-style: normal; font-size: 9px; font-weight: 700; letter-spacing: 0.04em; padding: 1px 5px; margin-inline-start: 6px; border-radius: 4px; background: color-mix(in srgb, var(--text) 16%, transparent); vertical-align: 1px; }
  .cn, .pn { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pst { width: 150px; display: flex; flex-direction: column; gap: 8px; text-align: start; border-radius: 12px; padding: 6px; }
  .pimg { position: relative; width: 100%; aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; background: var(--surface-2); display: grid; place-items: center; font-size: 40px; color: var(--muted); }
  .pimg img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .note { margin: 24px 0; }
  @media (max-width: 640px) { .hero { flex-direction: column; align-items: flex-start; padding: 20px; } }
</style>

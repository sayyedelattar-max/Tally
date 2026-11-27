<script lang="ts">
  import Rail from './components/Rail.svelte';
  import Player from './components/Player.svelte';
  import Search from './components/Search.svelte';
  import PlayerHost from './components/PlayerHost.svelte';
  import Home from './views/Home.svelte';
  import HomeReal from './views/HomeReal.svelte';
  import GuideReal from './views/GuideReal.svelte';
  import Live from './views/Live.svelte';
  import Browse from './views/Browse.svelte';
  import Replay from './views/Replay.svelte';
  import Playlists from './views/Playlists.svelte';
  import PlaylistsReal from './views/PlaylistsReal.svelte';
  import LiveReal from './views/LiveReal.svelte';
  import TitlesReal from './views/TitlesReal.svelte';
  import Settings from './views/Settings.svelte';
  import { onMount } from 'svelte';
  import { ui, t, toast, openSearch, loadRegistry } from './lib/store.svelte';
  import { inApp, api } from './lib/backend';
  import { move } from './lib/spatial';

  onMount(() => {
    loadRegistry().catch(() => {});
    // Providers change their lists now and then: re-download when the saved copy is older than 6 hours,
    // at start and again every 30 minutes while the app stays open. Failures stay silent.
    const tick = () => {
      if (!inApp) return;
      api.autoRefresh(6 * 3600).then((r) => {
        if (r) { toast(t('toast.updatedPl')); loadRegistry().catch(() => {}); }
      }).catch(() => {});
    };
    const first = setTimeout(tick, 4000);
    const iv = setInterval(tick, 30 * 60 * 1000);
    return () => { clearTimeout(first); clearInterval(iv); };
  });

  const sample = $derived(inApp && ['replay'].includes(ui.view));

  $effect(() => {
    const el = document.documentElement;
    el.lang = ui.lang;
    el.dir = ui.lang === 'ar' ? 'rtl' : 'ltr';
    el.dataset.accent = ui.accent;
  });

  const typing = (e: Event) => {
    const el = e.target as HTMLElement | null;
    return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
  };

  function onkey(e: KeyboardEvent) {
    if (e.defaultPrevented) return;
    const k = e.key;
    if ((e.ctrlKey || e.metaKey) && k.toLowerCase() === 'k') {
      e.preventDefault();
      if (!ui.searchOpen) openSearch();
      return;
    }
    if (k === '/' && !typing(e) && !ui.searchOpen && !ui.playing && !ui.rplay) {
      e.preventDefault();
      openSearch();
      return;
    }
    if (ui.playing || ui.rplay || typing(e) || e.altKey || e.ctrlKey || e.metaKey) return;
    const dirs: Record<string, 'left' | 'right' | 'up' | 'down'> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
    if (dirs[k] && move(dirs[k])) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="app">
  <div class="main-col">
    <Rail />
    <main id="main" class="main">
      {#if sample}<p class="sample mono" role="note">{t('real.sample')}</p>{/if}
      {#if ui.view === 'home'}{#if inApp}<HomeReal />{:else}<Home />{/if}
      {:else if ui.view === 'live'}{#if inApp}<LiveReal />{:else}<Live />{/if}
      {:else if ui.view === 'guide'}<GuideReal />
      {:else if ui.view === 'movies' || ui.view === 'series'}{#if inApp}{#key ui.view}<TitlesReal kind={ui.view === 'movies' ? 'movie' : 'series'} />{/key}{:else}<Browse kind={ui.view === 'movies' ? 'movie' : 'series'} />{/if}
      {:else if ui.view === 'replay'}<Replay />
      {:else if ui.view === 'playlists'}{#if inApp}<PlaylistsReal />{:else}<Playlists />{/if}
      {:else}<Settings />{/if}
    </main>
  </div>
</div>

{#if ui.searchOpen}<Search />{/if}
{#if ui.playing}<Player />{/if}
{#if ui.rplay}<PlayerHost />{/if}
{#if ui.toast}<div class="toast" role="status">{ui.toast}</div>{/if}

<style>
  .sample {
    margin: 4px 0 14px;
    padding: 10px 14px;
    border: 1px dashed var(--line-2);
    border-radius: 10px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
</style>

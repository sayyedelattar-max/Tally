<script lang="ts">
  import Rail from './components/Rail.svelte';
  import TopBar from './components/TopBar.svelte';
  import Player from './components/Player.svelte';
  import Search from './components/Search.svelte';
  import Home from './views/Home.svelte';
  import Live from './views/Live.svelte';
  import Browse from './views/Browse.svelte';
  import Replay from './views/Replay.svelte';
  import Playlists from './views/Playlists.svelte';
  import Settings from './views/Settings.svelte';
  import { ui, openSearch } from './lib/store.svelte';
  import { move } from './lib/spatial';

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
    if (k === '/' && !typing(e) && !ui.searchOpen && !ui.playing) {
      e.preventDefault();
      openSearch();
      return;
    }
    if (ui.playing || typing(e) || e.altKey || e.ctrlKey || e.metaKey) return;
    const dirs: Record<string, 'left' | 'right' | 'up' | 'down'> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
    if (dirs[k] && move(dirs[k])) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="app">
  <Rail />
  <div class="main-col">
    <TopBar />
    <main id="main" class="main">
      {#if ui.view === 'home'}<Home />
      {:else if ui.view === 'live'}<Live />
      {:else if ui.view === 'movies' || ui.view === 'series'}<Browse kind={ui.view === 'movies' ? 'movie' : 'series'} />
      {:else if ui.view === 'replay'}<Replay />
      {:else if ui.view === 'playlists'}<Playlists />
      {:else}<Settings />{/if}
    </main>
  </div>
</div>

{#if ui.searchOpen}<Search />{/if}
{#if ui.playing}<Player />{/if}
{#if ui.toast}<div class="toast" role="status">{ui.toast}</div>{/if}

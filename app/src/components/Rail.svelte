<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, t, go, openSearch } from '../lib/store.svelte';
  import { BRAND } from '../lib/brand';
  import type { View } from '../lib/types';

  const items: { view: View | 'search'; icon: string; key: string }[] = [
    { view: 'home', icon: 'home', key: 'nav.home' },
    { view: 'live', icon: 'tv', key: 'nav.live' },
    { view: 'movies', icon: 'film', key: 'nav.movies' },
    { view: 'series', icon: 'layers', key: 'nav.series' },
    { view: 'replay', icon: 'replay', key: 'nav.replay' },
    { view: 'playlists', icon: 'list', key: 'nav.playlists' },
    { view: 'search', icon: 'search', key: 'nav.search' },
    { view: 'settings', icon: 'sliders', key: 'nav.settings' },
  ];

  function pick(v: View | 'search') {
    if (v === 'search') openSearch();
    else go(v);
  }
</script>

<nav class="rail" aria-label="Main">
  <div class="brand"><span class="dot"></span><span class="display">{BRAND}</span></div>
  <ul>
    {#each items as it}
      <li>
        <button class="item" class:on={ui.view === it.view} aria-current={ui.view === it.view ? 'page' : undefined} onclick={() => pick(it.view)}>
          <Icon name={it.icon} size={24} />
          <span>{t(it.key)}</span>
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  .rail {
    background: var(--surface);
    border-inline-end: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
  }
  .brand {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 26px 0 20px;
    font-size: 24px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .item {
    position: relative;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 14px 4px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    text-align: center;
    transition: color 0.12s, background 0.12s;
  }
  .item:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .item.on {
    color: var(--text);
    background: var(--surface-2);
  }
  .item.on::before {
    content: '';
    position: absolute;
    inset-inline-start: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    background: var(--accent);
    border-radius: 0 3px 3px 0;
  }
  :global([dir='rtl']) .item.on::before {
    border-radius: 3px 0 0 3px;
  }
  .item:focus-visible {
    outline-offset: -3px;
  }
  @media (max-width: 760px) {
    .rail {
      flex-direction: row;
      border-inline-end: 0;
      border-top: 1px solid var(--line);
      overflow-x: auto;
      overflow-y: hidden;
    }
    .brand {
      display: none;
    }
    ul {
      flex-direction: row;
      width: 100%;
    }
    li {
      flex: 1 0 64px;
    }
    .item {
      padding: 10px 2px;
      font-size: 11px;
    }
    .item.on::before {
      inset-inline: 12px;
      top: 0;
      bottom: auto;
      width: auto;
      height: 3px;
      border-radius: 0 0 3px 3px;
    }
  }
</style>

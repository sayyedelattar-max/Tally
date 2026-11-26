<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, t, go, openSearch } from '../lib/store.svelte';
  import { BRAND, BRAND_SUFFIX } from '../lib/brand';
  import { inApp } from '../lib/backend';
  import { clock, TODAY } from '../lib/mock';
  import type { View } from '../lib/types';

  const tabs: { view: View; key: string }[] = [
    { view: 'home', key: 'nav.home' },
    { view: 'live', key: 'nav.live' },
    ...(inApp ? [{ view: 'guide' as View, key: 'nav.guide' }] : []),
    { view: 'movies', key: 'nav.movies' },
    { view: 'series', key: 'nav.series' },
    { view: 'replay', key: 'nav.replay' },
    { view: 'playlists', key: 'nav.playlists' },
  ];

  let tick = $state(Date.now());
  $effect(() => {
    if (!inApp) return;
    const i = setInterval(() => (tick = Date.now()), 15_000);
    return () => clearInterval(i);
  });
  const now = $derived(inApp ? new Date(tick) : TODAY);
  const hhmm = $derived(inApp ? `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}` : clock(0));
</script>

<header class="bar-top">
  <div class="brand"><b>{BRAND.charAt(0) + BRAND.slice(1).toLowerCase()}</b><span>{BRAND_SUFFIX}</span></div>
  <nav aria-label="Main" class="tabs">
    {#each tabs as it}
      <button class="tab" class:on={ui.view === it.view} aria-current={ui.view === it.view ? 'page' : undefined} onclick={() => go(it.view)}>{t(it.key)}</button>
    {/each}
  </nav>
  <div class="tools">
    <span class="clock ltr">{hhmm}</span>
    <button class="ico" onclick={openSearch} aria-label={t('nav.search')} title="{t('nav.search')} (Ctrl K)"><Icon name="search" size={20} /></button>
    <button class="ico" class:on={ui.view === 'settings'} onclick={() => go('settings')} aria-label={t('nav.settings')} title={t('nav.settings')}><Icon name="sliders" size={20} /></button>
    <button class="ico lang" onclick={() => (ui.lang = ui.lang === 'ar' ? 'en' : 'ar')} aria-label={t('top.langAria')}>{ui.lang === 'ar' ? 'EN' : 'ع'}</button>
  </div>
</header>

<style>
  .bar-top { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 16px; padding: 14px 32px; position: relative; z-index: 5; }
  .brand { display: flex; align-items: baseline; gap: 6px; }
  .brand b { font-size: 20px; font-weight: 700; letter-spacing: -0.01em; }
  .brand span { font-size: 11px; font-weight: 600; letter-spacing: 0.14em; color: var(--muted); }
  .tabs { display: flex; gap: 2px; padding: 4px; border-radius: 999px; background: color-mix(in srgb, var(--text) 7%, transparent); backdrop-filter: blur(20px); }
  .tab { padding: 8px 18px; border-radius: 999px; font-size: 15px; font-weight: 500; color: var(--muted); white-space: nowrap; transition: background 0.15s, color 0.15s; }
  .tab:hover { color: var(--text); }
  .tab.on { background: color-mix(in srgb, var(--text) 16%, transparent); color: var(--text); font-weight: 600; }
  .tab:focus-visible, .ico:focus-visible { outline: 2px solid var(--text); outline-offset: 2px; }
  .tools { display: flex; align-items: center; gap: 4px; justify-self: end; }
  .clock { font-size: 14px; font-weight: 500; color: var(--muted); margin-inline-end: 10px; font-variant-numeric: tabular-nums; }
  .ico { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; color: var(--muted); font-size: 13px; font-weight: 600; }
  .ico:hover, .ico.on { background: color-mix(in srgb, var(--text) 12%, transparent); color: var(--text); }
  @media (max-width: 900px) {
    .bar-top { grid-template-columns: 1fr auto; padding: 10px 16px; }
    .tabs { grid-column: 1 / -1; grid-row: 2; overflow-x: auto; justify-self: stretch; }
    .clock { display: none; }
  }
</style>

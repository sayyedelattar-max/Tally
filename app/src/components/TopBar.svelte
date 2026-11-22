<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, t, go, openSearch } from '../lib/store.svelte';
  import { playlists, daysLeft, clock, TODAY } from '../lib/mock';

  const pl = $derived(playlists.find((p) => p.id === ui.activePlaylist)!);
  const left = $derived(daysLeft(pl.expires));
  const date = $derived(TODAY.toLocaleDateString(ui.lang === 'ar' ? 'ar-EG' : 'en-GB', { weekday: 'short', day: 'numeric', month: 'short' }));
  const titles: Record<string, string> = {
    home: 'nav.home', live: 'nav.live', movies: 'nav.movies', series: 'nav.series',
    replay: 'nav.replay', playlists: 'nav.playlists', settings: 'nav.settings',
  };
</script>

<header class="top">
  <h1 class="display">{t(titles[ui.view])}</h1>
  <button class="field search" onclick={openSearch} aria-label={t('search.placeholder')}>
    <Icon name="search" size={18} />
    <span class="ph">{t('search.placeholder')}</span>
    <kbd class="mono ltr">Ctrl K</kbd>
  </button>
  <button class="plan" onclick={() => go('playlists')} aria-label={t('nav.playlists')}>
    <span class="name">{pl.name}</span>
    {#if left !== null}<span class="mono muted">{t('top.expires')} {pl.expires} · {left} {t('top.days')}</span>{/if}
  </button>
  <span class="mono clock ltr">{clock(0)} <span class="muted">· {date}</span></span>
  <button class="btn square lang" onclick={() => (ui.lang = ui.lang === 'ar' ? 'en' : 'ar')} aria-label={t('top.langAria')}>
    <span class="mono">{ui.lang === 'ar' ? 'EN' : 'ع'}</span>
  </button>
</header>

<style>
  .top {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px 32px 14px;
    flex-wrap: wrap;
  }
  h1 {
    font-size: 34px;
    margin-inline-end: auto;
    min-width: 120px;
  }
  .search {
    width: min(360px, 100%);
    text-align: start;
    cursor: pointer;
  }
  .search:hover {
    border-color: var(--muted);
  }
  .ph {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 6px;
    border: 1px solid var(--line-2);
    border-radius: 4px;
  }
  .plan {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 4px 12px;
    border-radius: var(--radius);
    line-height: 1.25;
  }
  .plan:hover {
    background: var(--surface-2);
  }
  .plan .name {
    font-weight: 600;
    font-size: 14px;
  }
  .clock {
    font-size: 14px;
    color: var(--text);
  }
  .lang {
    font-size: 15px;
  }
  @media (max-width: 760px) {
    .top {
      padding: 14px 16px 8px;
      gap: 10px;
    }
    h1 {
      font-size: 28px;
    }
    .plan,
    .clock {
      display: none;
    }
    kbd {
      display: none;
    }
    .search {
      order: 3;
      width: 100%;
    }
  }
</style>

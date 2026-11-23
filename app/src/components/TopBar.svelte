<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, t, go, openSearch, realActive } from '../lib/store.svelte';
  import { inApp, expiry } from '../lib/backend';
  import { playlists, daysLeft, clock, TODAY } from '../lib/mock';

  const mock = $derived(playlists.find((p) => p.id === ui.activePlaylist)!);
  const real = $derived(inApp ? realActive() : null);
  const ex = $derived(real ? expiry(real.expiresAt) : null);
  const planName = $derived(inApp ? (real?.name ?? t('pl.none')) : mock.name);
  const planDate = $derived(inApp ? (ex?.date ?? null) : mock.expires);
  const planLeft = $derived(inApp ? (ex?.days ?? null) : daysLeft(mock.expires));

  let tick = $state(Date.now());
  $effect(() => {
    if (!inApp) return;
    const i = setInterval(() => (tick = Date.now()), 15_000);
    return () => clearInterval(i);
  });
  const now = $derived(inApp ? new Date(tick) : TODAY);
  const hhmm = $derived(inApp ? `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}` : clock(0));
  const date = $derived(now.toLocaleDateString(ui.lang === 'ar' ? 'ar-EG' : 'en-GB', { weekday: 'short', day: 'numeric', month: 'short' }));
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
    <span class="name">{planName}</span>
    {#if planLeft !== null}<span class="mono muted">{t('top.expires')} {planDate} · {planLeft} {t('top.days')}</span>{/if}
  </button>
  <span class="mono clock ltr">{hhmm} <span class="muted">· {date}</span></span>
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

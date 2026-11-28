<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import SettingRow from '../components/SettingRow.svelte';
  import PlaylistForm from '../components/PlaylistForm.svelte';
  import { ui, t, toast, go, realActive, loadRegistry } from '../lib/store.svelte';
  import { api, inApp, expiry } from '../lib/backend';
  import { liveCategories, movieCategories, playlists, seriesCategories, daysLeft } from '../lib/mock';
  import type { Accent } from '../lib/types';
  import { view, saveView } from '../lib/view.svelte';

  const pl = $derived(playlists.find((p) => p.id === ui.activePlaylist)!);
  const left = $derived(daysLeft(pl.expires));
  const real = $derived(inApp ? realActive() : null);
  const rex = $derived(real ? expiry(real.expiresAt) : null);
  let editing = $state(false);
  let refreshing = $state(false);
  let refreshError = $state('');
  async function refreshReal() {
    if (!real) return;
    refreshing = true;
    refreshError = '';
    try {
      await api.refresh(real.id);
      await loadRegistry();
      toast(t('toast.refreshed'));
    } catch (e) {
      refreshError = String(e);
    } finally {
      refreshing = false;
    }
  }
  let hideTab = $state<'live' | 'movies' | 'series'>('live');

  const cats = $derived(hideTab === 'live' ? liveCategories : hideTab === 'movies' ? movieCategories : seriesCategories);
  const label = (c: string) => (hideTab === 'live' ? t('cat.' + c) : c);
  const isHidden = (c: string) => ui.hidden[hideTab].includes(c);
  function flip(c: string) {
    const list = ui.hidden[hideTab];
    const i = list.indexOf(c);
    if (i >= 0) list.splice(i, 1);
    else list.push(c);
  }

  const accents: { id: Accent; color: string }[] = [
    { id: 'mono', color: '#f2f2ee' },
    { id: 'teal', color: '#4fc3a8' },
    { id: 'violet', color: '#b3a8ff' },
    { id: 'amber', color: '#f2b13c' },
  ];
</script>

<div class="wrap">
  {#if inApp}
    <section>
      <h2 class="display">{t('set.connection')}</h2>
      {#if editing && real}
        {#key real.id}<PlaylistForm editing={real} ondone={() => (editing = false)} oncancel={() => (editing = false)} />{/key}
      {:else if real}
        <SettingRow label={real.name} desc={`Xtream Codes · ${real.host.replace(/^https?:\/\//, '')} · ${real.user}`}>
          <span class="pill">{real.status}</span>
          {#if rex}<span class="mono muted">{t('top.expires')} {rex.date} · {rex.days} {t('top.days')}</span>{/if}
        </SettingRow>
        {#if real.maxConnections}
          <SettingRow label={t('set.connections')} desc={t('set.connectionsDesc')}>
            <span class="mono">{real.activeConnections ?? 0} / {real.maxConnections}</span>
          </SettingRow>
        {/if}
        <SettingRow label={`${real.channels} ${t('real.channels')}`} desc={t('set.refreshDesc')}>
          <button class="btn" disabled={refreshing} onclick={refreshReal}><Icon name="refresh" size={16} /> {refreshing ? t('pl.refreshing') : t('pl.refresh')}</button>
          <button class="btn" onclick={() => (editing = true)}><Icon name="sliders" size={16} /> {t('pl.edit')}</button>
          <button class="btn" onclick={() => go('playlists')}>{t('pl.manage')}</button>
        </SettingRow>
        {#if refreshError}<p class="err" role="alert">{refreshError}</p>{/if}
      {:else}
        <SettingRow label={t('real.noPlaylist')} desc={t('real.noPlaylistLead')}>
          <button class="btn primary" onclick={() => go('playlists')}><Icon name="plus" size={16} /> {t('pl.add')}</button>
        </SettingRow>
      {/if}
    </section>
  {:else}
  <section>
    <h2 class="display">{t('set.account')}</h2>
    <SettingRow label={pl.name} desc={`${pl.type === 'xtream' ? 'Xtream Codes' : 'M3U'} · ${pl.host}`}>
      <span class="pill">{pl.status}</span>
      {#if left !== null}<span class="mono muted">{t('top.expires')} {pl.expires} · {left} {t('top.days')}</span>{/if}
    </SettingRow>
    {#if pl.connections}
      <SettingRow label={t('set.connections')} desc={t('set.connectionsDesc')}>
        <span class="mono">{pl.connections[0]} / {pl.connections[1]}</span>
      </SettingRow>
    {/if}
  </section>
  {/if}

  <section>
    <h2 class="display">{t('set.playback')}</h2>
    <SettingRow label={t('set.format')} desc={t('set.formatDesc')}>
      <div class="seg" role="group" aria-label={t('set.format')}>
        <button class:on={ui.settings.format === 'ts'} onclick={() => (ui.settings.format = 'ts')}>MPEG-TS</button>
        <button class:on={ui.settings.format === 'hls'} onclick={() => (ui.settings.format = 'hls')}>HLS</button>
      </div>
    </SettingRow>
    <SettingRow label={t('set.hw')} desc={t('set.hwDesc')}>
      <div class="seg" role="group" aria-label={t('set.hw')}>
        <button class:on={ui.settings.hw} onclick={() => (ui.settings.hw = true)}>{t('common.on')}</button>
        <button class:on={!ui.settings.hw} onclick={() => (ui.settings.hw = false)}>{t('common.off')}</button>
      </div>
    </SettingRow>
    {#if inApp}
      <SettingRow label={t('set.engine')} desc={t('set.engineDesc')}>
        <div class="seg" role="group" aria-label={t('set.engine')}>
          {#each ['auto', 'hq', 'builtin'] as e}
            <button class:on={ui.settings.engine === e} onclick={() => (ui.settings.engine = e)}>{t('set.engine.' + e)}</button>
          {/each}
        </div>
      </SettingRow>
      <SettingRow label={t('set.quality')} desc={t('set.qualityDesc')}>
        <div class="seg" role="group" aria-label={t('set.quality')}>
          {#each ['standard', 'high', 'ultra'] as q}
            <button class:on={ui.settings.quality === q} onclick={() => (ui.settings.quality = q)}>{t('set.quality.' + q)}</button>
          {/each}
        </div>
      </SettingRow>
    {/if}
    <SettingRow label={t('set.buffer')} desc={t('set.bufferDesc')}>
      <div class="seg" role="group" aria-label={t('set.buffer')}>
        {#each ['low', 'balanced', 'stable'] as b}
          <button class:on={ui.settings.buffer === b} onclick={() => (ui.settings.buffer = b)}>{t('set.buffer.' + b)}</button>
        {/each}
      </div>
    </SettingRow>
    <SettingRow label={t('set.subs')} desc={t('set.subsDesc')}>
      <div class="seg" role="group" aria-label={t('set.subs')}>
        {#each ['small', 'medium', 'large'] as s}
          <button class:on={ui.settings.subSize === s} onclick={() => (ui.settings.subSize = s)}>{t('set.size.' + s)}</button>
        {/each}
      </div>
    </SettingRow>
  </section>

  <section>
    <h2 class="display">{t('set.liveView')}</h2>
    <SettingRow label={t('set.defView')} desc={t('set.defViewDesc')}>
      <div class="seg" role="group" aria-label={t('set.defView')}>
        <button class:on={view.channels} onclick={() => { view.channels = true; saveView(); }}>{t('mrg.channels')}</button>
        <button class:on={!view.channels} onclick={() => { view.channels = false; saveView(); }}>{t('mrg.provider')}</button>
      </div>
    </SettingRow>
    <SettingRow label={t('set.foldGroups')} desc={t('set.foldGroupsDesc')}>
      <div class="seg" role="group" aria-label={t('set.foldGroups')}>
        <button class:on={view.fold} onclick={() => { view.fold = true; saveView(); }}>{t('common.on')}</button>
        <button class:on={!view.fold} onclick={() => { view.fold = false; saveView(); }}>{t('common.off')}</button>
      </div>
    </SettingRow>
    <SettingRow label={t('set.mergeLines')} desc={t('set.mergeLinesDesc')}>
      <div class="seg" role="group" aria-label={t('set.mergeLines')}>
        <button class:on={view.lines} onclick={() => { view.lines = true; saveView(); }}>{t('common.on')}</button>
        <button class:on={!view.lines} onclick={() => { view.lines = false; saveView(); }}>{t('common.off')}</button>
      </div>
    </SettingRow>
  </section>

  <section>
    <h2 class="display">{t('set.content')}</h2>
    <SettingRow label={t('set.parental')} desc={t('set.parentalDesc')}>
      <div class="seg" role="group" aria-label={t('set.parental')}>
        <button class:on={ui.settings.parental} onclick={() => { ui.settings.parental = true; toast(t('toast.pinNote')); }}><Icon name="lock" size={16} /> {t('common.on')}</button>
        <button class:on={!ui.settings.parental} onclick={() => (ui.settings.parental = false)}>{t('common.off')}</button>
      </div>
    </SettingRow>
    <div class="hide">
      <div class="hh">
        <div class="txt"><b>{t('set.hide')}</b><span class="muted">{t('set.hideDesc')}</span></div>
        <div class="seg" role="group" aria-label={t('set.hide')}>
          <button class:on={hideTab === 'live'} onclick={() => (hideTab = 'live')}>{t('nav.live')}</button>
          <button class:on={hideTab === 'movies'} onclick={() => (hideTab = 'movies')}>{t('nav.movies')}</button>
          <button class:on={hideTab === 'series'} onclick={() => (hideTab = 'series')}>{t('nav.series')}</button>
        </div>
      </div>
      <div class="chips wrapchips">
        {#each cats as c}
          <button class="chip" class:on={!isHidden(c)} aria-pressed={!isHidden(c)} onclick={() => flip(c)}>
            <Icon name={isHidden(c) ? 'eyeoff' : 'check'} size={16} /> {label(c)}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section>
    <h2 class="display">{t('set.appearance')}</h2>
    <SettingRow label={t('set.language')} desc={t('set.languageDesc')}>
      <div class="seg" role="group" aria-label={t('set.language')}>
        <button class:on={ui.lang === 'en'} onclick={() => (ui.lang = 'en')}>English</button>
        <button class:on={ui.lang === 'ar'} onclick={() => (ui.lang = 'ar')}>العربية</button>
      </div>
    </SettingRow>
    <SettingRow label={t('set.accent')} desc={t('set.accentDesc')}>
      <div class="swatches" role="group" aria-label={t('set.accent')}>
        {#each accents as a}
          <button class="sw" class:on={ui.accent === a.id} aria-pressed={ui.accent === a.id} aria-label={t('accent.' + a.id)} onclick={() => (ui.accent = a.id)}>
            <span style="background:{a.color}"></span>{t('accent.' + a.id)}
          </button>
        {/each}
      </div>
    </SettingRow>
  </section>

  <section>
    <h2 class="display">{t('set.data')}</h2>
    <SettingRow label={t('set.clearCache')} desc={t('set.clearCacheDesc')}>
      <button class="btn danger" onclick={() => toast(t('toast.cacheCleared'))}><Icon name="trash" size={16} /> {t('set.clear')}</button>
    </SettingRow>
    <SettingRow label={t('set.clearMovies')}>
      <button class="btn danger" onclick={() => toast(t('toast.moviesCleared'))}><Icon name="trash" size={16} /> {t('set.clear')}</button>
    </SettingRow>
    <SettingRow label={t('set.clearSeries')}>
      <button class="btn danger" onclick={() => toast(t('toast.seriesCleared'))}><Icon name="trash" size={16} /> {t('set.clear')}</button>
    </SettingRow>
  </section>
</div>

<style>
  .wrap {
    max-width: 860px;
    display: flex;
    flex-direction: column;
    gap: 34px;
    padding-top: 10px;
  }
  h2 {
    font-size: 22px;
    font-weight: 700;
    margin: 0 4px 12px;
    color: var(--text);
  }
  section > :global(.srow) { background: var(--surface); padding-inline: 22px; border-bottom: 1px solid var(--line); }
  section > :global(h2 + .srow) { border-start-start-radius: 20px; border-start-end-radius: 20px; }
  section > :global(.srow:last-child) { border-end-start-radius: 20px; border-end-end-radius: 20px; border-bottom: 0; }
  .err { color: var(--live); overflow-wrap: anywhere; }
  .hide {
    padding: 14px 0 6px;
    border-bottom: 1px solid var(--line);
  }
  .hh {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    flex-wrap: wrap;
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .txt b {
    font-weight: 600;
  }
  .txt .muted {
    font-size: 13px;
  }
  .wrapchips {
    flex-wrap: wrap;
    padding-top: 14px;
  }
  .swatches {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .sw {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 0 16px 0 12px;
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    font-weight: 500;
  }
  .sw span {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: block;
  }
  .sw:hover {
    background: var(--surface-2);
  }
  .sw.on {
    border-color: var(--text);
    background: var(--surface-3);
  }
</style>

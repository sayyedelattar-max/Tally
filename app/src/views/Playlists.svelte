<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { ui, t, toast } from '../lib/store.svelte';
  import { daysLeft, playlists } from '../lib/mock';

  let adding = $state(false);
  let kind = $state<'xtream' | 'm3u'>('xtream');
  let name = $state('');
  let host = $state('');
  let user = $state('');
  let pass = $state('');
  let busy = $state(false);
  let error = $state('');

  type Connected = { status: string; expiresAt: number | null; trial: boolean; maxConnections: number | null; activeConnections: number | null; channels: number };
  const inApp = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  async function save(e: Event) {
    e.preventDefault();
    if (!inApp || kind !== 'xtream') {
      adding = false;
      toast(t('toast.plNote'));
      return;
    }
    busy = true;
    error = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const r = await invoke<Connected>('connect_xtream', { host: host.trim(), user: user.trim(), pass });
      toast(`${t('pl.connected')} · ${r.channels} ${t('nav.live')}`);
      adding = false;
      pass = '';
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="head">
  <p class="muted">{t('pl.lead')}</p>
  <button class="btn primary" onclick={() => (adding = !adding)}><Icon name={adding ? 'x' : 'plus'} size={18} /> {adding ? t('common.cancel') : t('pl.add')}</button>
</div>

{#if adding}
  <form class="add" onsubmit={save}>
    <div class="seg" role="group" aria-label={t('pl.type')}>
      <button type="button" class:on={kind === 'xtream'} onclick={() => (kind = 'xtream')}>Xtream Codes</button>
      <button type="button" class:on={kind === 'm3u'} onclick={() => (kind = 'm3u')}>M3U / URL</button>
    </div>
    <label class="f"><span>{t('pl.name')}</span><input class="inp" bind:value={name} placeholder="Home subscription" /></label>
    <label class="f"><span>{kind === 'xtream' ? t('pl.server') : t('pl.url')}</span><input class="inp ltr-in" dir="ltr" bind:value={host} placeholder={kind === 'xtream' ? 'https://provider.example' : 'https://example.com/list.m3u'} /></label>
    {#if kind === 'xtream'}
      <div class="two">
        <label class="f"><span>{t('pl.user')}</span><input class="inp ltr-in" dir="ltr" bind:value={user} autocomplete="off" /></label>
        <label class="f"><span>{t('pl.pass')}</span><input class="inp ltr-in" dir="ltr" type="password" bind:value={pass} autocomplete="off" /></label>
      </div>
    {/if}
    <p class="muted note"><Icon name="lock" size={14} /> {t('pl.secure')}</p>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <div><button class="btn primary" type="submit" disabled={busy}>{busy ? t('pl.connecting') : t('pl.save')}</button></div>
  </form>
{/if}

<ul class="cards">
  {#each playlists as p (p.id)}
    {@const left = daysLeft(p.expires)}
    <li class="pc" class:act={p.id === ui.activePlaylist}>
      <div class="top">
        <div>
          <h2 class="display">{p.name}</h2>
          <span class="mono muted ltr">{p.type === 'xtream' ? 'Xtream Codes' : 'M3U'} · {p.host}</span>
        </div>
        {#if p.id === ui.activePlaylist}<span class="pill act-pill">{t('pl.active')}</span>{/if}
      </div>
      <dl>
        <div><dt>{t('pl.status')}</dt><dd>{p.status}</dd></div>
        <div><dt>{t('top.expires')}</dt><dd class="mono ltr">{p.expires ?? '—'}{#if left !== null} · {left} {t('top.days')}{/if}</dd></div>
        <div><dt>{t('pl.connections')}</dt><dd class="mono">{p.connections ? `${p.connections[0]} / ${p.connections[1]}` : '—'}</dd></div>
        <div><dt>{t('pl.updated')}</dt><dd class="mono">{p.updated}</dd></div>
      </dl>
      <div class="counts mono">
        <span><b>{p.counts.live}</b> {t('nav.live')}</span>
        <span><b>{p.counts.movies}</b> {t('nav.movies')}</span>
        <span><b>{p.counts.series}</b> {t('nav.series')}</span>
      </div>
      <div class="acts">
        {#if p.id !== ui.activePlaylist}
          <button class="btn primary" onclick={() => { ui.activePlaylist = p.id; toast(t('toast.switched')); }}>{t('pl.switch')}</button>
        {/if}
        <button class="btn" onclick={() => toast(t('toast.refreshed'))}><Icon name="refresh" size={16} /> {t('pl.refresh')}</button>
        <button class="btn danger" onclick={() => toast(t('toast.plNote'))}><Icon name="trash" size={16} /> {t('pl.remove')}</button>
      </div>
    </li>
  {/each}
</ul>

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin: 10px 0 22px;
  }
  .head p {
    max-width: 62ch;
  }
  .add {
    display: flex;
    flex-direction: column;
    gap: 16px;
    align-items: flex-start;
    max-width: 620px;
    padding: 22px;
    margin-bottom: 26px;
    border: 1px solid var(--line-2);
    border-radius: 14px;
    background: var(--surface);
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    font-size: 13px;
    color: var(--muted);
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    width: 100%;
  }
  .inp {
    min-height: 46px;
    padding: 0 14px;
    border: 1px solid var(--line-2);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--text);
    font-size: 15px;
  }
  .inp:focus-visible {
    outline: none;
    border-color: var(--accent);
  }
  .err {
    margin: 0;
    color: var(--live);
    font-size: 14px;
    overflow-wrap: anywhere;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 20px;
  }
  .pc {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 24px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface);
  }
  .pc.act {
    border-color: var(--accent);
  }
  .top {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: flex-start;
  }
  h2 {
    font-size: 30px;
    font-weight: 600;
  }
  .act-pill {
    color: var(--accent);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px 20px;
  }
  dt {
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 2px;
  }
  dd {
    margin: 0;
    font-weight: 500;
  }
  .counts {
    display: flex;
    gap: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    color: var(--muted);
  }
  .counts b {
    color: var(--text);
    font-weight: 500;
  }
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: auto;
  }
</style>

<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import PlaylistForm from '../components/PlaylistForm.svelte';
  import { ui, t, toast, loadRegistry, go } from '../lib/store.svelte';
  import { api, expiry, type PlaylistInfo } from '../lib/backend';

  let form = $state<'none' | 'add' | string>('none');
  let busyId = $state('');
  let confirmId = $state('');
  let failure = $state('');

  const editing = $derived(ui.real.items.find((p) => p.id === form) ?? null);
  const when = (s: number) => new Date(s * 1000).toLocaleString(ui.lang === 'ar' ? 'ar-EG' : 'en-GB', { dateStyle: 'medium', timeStyle: 'short' });

  async function run(id: string, fn: () => Promise<unknown>, done: string) {
    busyId = id;
    failure = '';
    try {
      await fn();
      await loadRegistry();
      toast(done);
    } catch (e) {
      failure = String(e);
    } finally {
      busyId = '';
    }
  }

  const refresh = (p: PlaylistInfo) => run(p.id, () => api.refresh(p.id), t('toast.refreshed'));
  const use = (p: PlaylistInfo) => run(p.id, () => api.activate(p.id), t('toast.switched'));
  function remove(p: PlaylistInfo) {
    if (confirmId !== p.id) {
      confirmId = p.id;
      return;
    }
    confirmId = '';
    run(p.id, () => api.remove(p.id), t('pl.removed'));
  }
</script>

<div class="head">
  <p class="muted">{t('pl.lead')}</p>
  {#if form === 'none'}<button class="btn primary" onclick={() => (form = 'add')}><Icon name="plus" size={18} /> {t('pl.add')}</button>{/if}
</div>

{#if form === 'add'}
  <PlaylistForm ondone={() => (form = 'none')} oncancel={() => (form = 'none')} />
{:else if editing}
  {#key editing.id}<PlaylistForm {editing} ondone={() => (form = 'none')} oncancel={() => (form = 'none')} />{/key}
{/if}

{#if failure}<p class="err" role="alert">{failure}</p>{/if}

{#if ui.real.loaded && !ui.real.items.length && form === 'none'}
  <div class="none">
    <h2 class="display">{t('pl.empty')}</h2>
    <p class="muted">{t('real.noPlaylistLead')}</p>
    <button class="btn primary" onclick={() => (form = 'add')}><Icon name="plus" size={18} /> {t('pl.add')}</button>
  </div>
{/if}

<ul class="cards">
  {#each ui.real.items as p (p.id)}
    {@const ex = expiry(p.expiresAt)}
    {@const isActive = p.id === ui.real.active}
    <li class="pc" class:act={isActive}>
      <div class="top">
        <div>
          <h2 class="display">{p.name}</h2>
          <span class="mono muted ltr">Xtream Codes · {p.host.replace(/^https?:\/\//, '')}</span>
        </div>
        {#if isActive}<span class="pill act-pill">{t('pl.active')}</span>{/if}
      </div>
      <dl>
        <div><dt>{t('pl.status')}</dt><dd>{p.status}{p.trial ? ' · trial' : ''}</dd></div>
        <div><dt>{t('top.expires')}</dt><dd class="mono ltr">{ex ? ex.date : '—'}{#if ex} · {ex.days} {t('top.days')}{/if}</dd></div>
        <div><dt>{t('pl.connections')}</dt><dd class="mono">{p.maxConnections ? `${p.activeConnections ?? 0} / ${p.maxConnections}` : '—'}</dd></div>
        <div><dt>{t('pl.updated')}</dt><dd class="mono">{when(p.updatedAt)}</dd></div>
      </dl>
      <div class="counts mono"><span><b>{p.channels}</b> {t('real.channels')}</span><span class="ltr muted">{p.user}</span></div>
      <div class="acts">
        {#if !isActive}<button class="btn primary" disabled={!!busyId} onclick={() => use(p)}>{t('pl.switch')}</button>{/if}
        <button class="btn" disabled={!!busyId} onclick={() => refresh(p)}><Icon name="refresh" size={16} /> {busyId === p.id ? t('pl.refreshing') : t('pl.refresh')}</button>
        <button class="btn" disabled={!!busyId} onclick={() => { form = p.id; confirmId = ''; }}><Icon name="sliders" size={16} /> {t('pl.edit')}</button>
        <button class="btn danger" disabled={!!busyId} onclick={() => remove(p)}><Icon name="trash" size={16} /> {confirmId === p.id ? t('pl.confirmRemove') : t('pl.remove')}</button>
      </div>
      {#if isActive}<button class="link" onclick={() => go('live')}>{t('real.openLive')} <Icon name="chevron" size={16} flip /></button>{/if}
    </li>
  {/each}
</ul>

<style>
  .head { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; margin: 10px 0 22px; }
  .head p { max-width: 62ch; }
  .err { color: var(--live); overflow-wrap: anywhere; }
  .none { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; padding: 40px 0; }
  .none h2 { font-size: 32px; }
  .cards { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 20px; }
  .pc { display: flex; flex-direction: column; gap: 18px; padding: 24px; border: 1px solid var(--line); border-radius: 14px; background: var(--surface); min-width: 0; }
  .pc.act { border-color: var(--accent); }
  .top { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  h2 { font-size: 30px; font-weight: 600; overflow-wrap: anywhere; }
  .act-pill { color: var(--accent); }
  dl { margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 14px 20px; }
  dt { font-size: 12px; color: var(--muted); margin-bottom: 2px; }
  dd { margin: 0; font-weight: 500; }
  .counts { display: flex; justify-content: space-between; gap: 18px; padding-top: 14px; border-top: 1px solid var(--line); color: var(--muted); flex-wrap: wrap; }
  .counts b { color: var(--text); font-weight: 500; }
  .acts { display: flex; gap: 8px; flex-wrap: wrap; margin-top: auto; }
  .link { align-self: flex-start; display: inline-flex; align-items: center; gap: 4px; color: var(--accent); padding: 4px 0; }
</style>

<script lang="ts">
  import Icon from './Icon.svelte';
  import { t, toast, loadRegistry } from '../lib/store.svelte';
  import { api, type PlaylistInfo } from '../lib/backend';

  let { editing = null, ondone, oncancel }: { editing?: PlaylistInfo | null; ondone: () => void; oncancel: () => void } = $props();

  // The form is re-created per playlist (callers use {#key}), so the initial value is what we want.
  // svelte-ignore state_referenced_locally
  let name = $state(editing?.name ?? '');
  // svelte-ignore state_referenced_locally
  let host = $state(editing?.host ?? '');
  // svelte-ignore state_referenced_locally
  let user = $state(editing?.user ?? '');
  let pass = $state('');
  let busy = $state(false);
  let error = $state('');

  async function save(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const r = await api.save({ id: editing?.id ?? null, name: name.trim(), host: host.trim(), user: user.trim(), pass: pass || null });
      pass = '';
      await loadRegistry();
      toast(`${t('pl.saved')} · ${r.channels} ${t('real.channels')}`);
      ondone();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<form class="add" onsubmit={save}>
  <h2 class="display">{editing ? t('pl.editTitle') : t('pl.add')}</h2>
  <label class="f"><span>{t('pl.name')}</span><input class="inp" bind:value={name} placeholder="Home subscription" /></label>
  <label class="f"><span>{t('pl.server')}</span><input class="inp ltr-in" dir="ltr" bind:value={host} placeholder="http://provider.example" required /></label>
  <div class="two">
    <label class="f"><span>{t('pl.user')}</span><input class="inp ltr-in" dir="ltr" bind:value={user} autocomplete="off" required /></label>
    <label class="f"><span>{t('pl.pass')}</span><input class="inp ltr-in" dir="ltr" type="password" bind:value={pass} autocomplete="off" placeholder={editing ? t('pl.keepPass') : ''} required={!editing} /></label>
  </div>
  <p class="muted note"><Icon name="lock" size={14} /> {t('pl.secure')}</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  <div class="acts">
    <button class="btn primary" type="submit" disabled={busy}>{busy ? t('pl.connecting') : editing ? t('pl.saveChanges') : t('pl.save')}</button>
    <button class="btn" type="button" onclick={oncancel} disabled={busy}>{t('common.cancel')}</button>
  </div>
</form>

<style>
  .add { display: flex; flex-direction: column; gap: 16px; align-items: flex-start; max-width: 620px; padding: 22px; margin-bottom: 26px; border: 1px solid var(--line-2); border-radius: 14px; background: var(--surface); }
  h2 { font-size: 26px; font-weight: 600; }
  .f { display: flex; flex-direction: column; gap: 6px; width: 100%; font-size: 13px; color: var(--muted); }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; width: 100%; }
  .inp { min-height: 46px; padding: 0 14px; border: 1px solid var(--line-2); border-radius: var(--radius); background: var(--bg); color: var(--text); font-size: 15px; min-width: 0; }
  .inp:focus-visible { outline: none; border-color: var(--accent); }
  .note { display: flex; align-items: center; gap: 8px; font-size: 13px; margin: 0; }
  .err { margin: 0; color: var(--live); font-size: 14px; overflow-wrap: anywhere; }
  .acts { display: flex; gap: 8px; flex-wrap: wrap; }
  @media (max-width: 560px) { .two { grid-template-columns: 1fr; } }
</style>

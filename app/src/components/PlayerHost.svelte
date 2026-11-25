<script lang="ts">
  import { onMount } from 'svelte';
  import RealPlayer from './RealPlayer.svelte';
  import { ui, t } from '../lib/store.svelte';
  import { api, onHq } from '../lib/backend';

  // 'pending' while deciding, 'hq' when the high-quality player window has taken over, else the built-in player.
  let mode = $state<'pending' | 'hq' | 'builtin'>('pending');
  let note = $state('');
  let off: (() => void) | undefined;

  const BUFFER_SECS: Record<string, number> = { low: 10, balanced: 30, stable: 60 };

  async function toHq(): Promise<boolean> {
    const rp = ui.rplay;
    if (!rp) return false;
    try {
      await api.hqPlay(
        rp.list.map((c) => ({ name: c.name, liveId: c.vod ? null : c.id, vod: c.vod ?? null })),
        rp.index,
        { quality: ui.settings.quality, bufferSecs: BUFFER_SECS[ui.settings.buffer] ?? 30, fullscreen: false },
      );
      return true;
    } catch (e) {
      note = String(e);
      return false;
    }
  }

  onMount(() => {
    (async () => {
      const want = ui.settings.engine !== 'builtin';
      if (want && (await api.hqAvailable().catch(() => false))) {
        off = await onHq({
          closed: () => { ui.rplay = null; },
          index: (i) => { if (ui.rplay) ui.rplay.index = i; },
          error: (m) => { note = m; },
        });
        if (await toHq()) {
          mode = 'hq';
          return;
        }
        off?.();
        off = undefined;
      } else if (ui.settings.engine === 'hq') {
        note = t('hq.missing');
      }
      mode = 'builtin';
    })();
    return () => off?.();
  });

  function useBuiltin() {
    api.hqStop().catch(() => {});
    off?.();
    off = undefined;
    mode = 'builtin';
  }
  function stopHq() {
    api.hqStop().catch(() => {});
  }
  const cur = $derived(ui.rplay?.list[ui.rplay.index]);
</script>

{#if mode === 'builtin'}
  <RealPlayer />
{:else}
  <div class="host" role="dialog" aria-label={cur?.name ?? ''} data-overlay>
    {#if mode === 'pending'}
      <p class="muted" role="status">{t('rp.buffering')}</p>
    {:else}
      <div class="card">
        <span class="mono muted">{t('hq.title')}</span>
        <h2 class="display">{cur?.name}</h2>
        <p class="muted">{t('hq.lead')}</p>
        {#if note}<p class="mono err">{note}</p>{/if}
        <div class="row">
          <button class="btn primary" onclick={stopHq}>{t('hq.stop')}</button>
          <button class="btn" onclick={useBuiltin}>{t('hq.builtin')}</button>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .host { position: fixed; inset: 0; z-index: 60; background: var(--bg); display: grid; place-items: center; padding: 24px; }
  .card { max-width: 560px; display: flex; flex-direction: column; gap: 12px; }
  .card h2 { margin: 0; font-size: 34px; overflow-wrap: anywhere; }
  .card p { margin: 0; line-height: 1.6; }
  .row { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 8px; }
  .err { color: var(--live); overflow-wrap: anywhere; }
</style>

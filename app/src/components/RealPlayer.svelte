<script lang="ts">
  import Hls from 'hls.js';
  import mpegts from 'mpegts.js';
  import Icon from './Icon.svelte';
  import { ui, t } from '../lib/store.svelte';
  import { api } from '../lib/backend';

  let video: HTMLVideoElement | undefined = $state();
  let root: HTMLElement | undefined = $state();
  let status = $state<'loading' | 'playing' | 'buffering' | 'error'>('loading');
  let errKind = $state<'codec' | 'other'>('other');
  let errDetail = $state('');
  let paused = $state(false);
  let muted = $state(false);
  let idle = $state(false);
  let full = $state(false);
  let idleTimer: ReturnType<typeof setTimeout>;
  let hls: Hls | null = null;
  let ts: ReturnType<typeof mpegts.createPlayer> | null = null;
  let report = $state('');
  let diagBusy = $state(false);
  let ticket = 0;

  const rp = $derived(ui.rplay!);
  const cur = $derived(rp.list[rp.index]);

  function stop() {
    hls?.destroy();
    hls = null;
    try { ts?.destroy(); } catch {}
    ts = null;
    if (video) {
      video.removeAttribute('src');
      video.load();
    }
  }

  function fail(detail: string) {
    status = 'error';
    errDetail = detail;
    errKind = /codec|incompatible|bufferAppend|fragParsing/i.test(detail) ? 'codec' : 'other';
  }

  // Playback buffer presets (Settings → Playback): seconds to keep ahead, and playlist entries to stay behind the live edge.
  const BUFFER = { low: { ahead: 10, live: 3, stash: 384 }, balanced: { ahead: 30, live: 5, stash: 1536 }, stable: { ahead: 60, live: 8, stash: 4096 } };

  // Some providers answer a playlist address with a raw MPEG-TS stream; play that directly.
  function startTs(url: string, mine: number) {
    if (!video || !mpegts.isSupported()) return fail('manifest: not a playlist');
    const p = mpegts.createPlayer({ type: 'mpegts', isLive: true, url }, { enableWorker: true, enableStashBuffer: true, stashInitialSize: (BUFFER[ui.settings.buffer as keyof typeof BUFFER] ?? BUFFER.balanced).stash * 1024 / 1024, liveBufferLatencyChasing: false, lazyLoad: false });
    p.on(mpegts.Events.ERROR, (type: string, detail: string) => { if (mine === ticket) fail(`${type}: ${detail}`); });
    ts = p;
    p.attachMediaElement(video);
    p.load();
    p.play()?.catch?.(() => {});
  }

  async function diagnose() {
    diagBusy = true;
    try { report = await api.diagnose(cur.id); } catch (e) { report = String(e); }
    diagBusy = false;
  }

  async function start(id: number) {
    report = '';
    const mine = ++ticket;
    stop();
    status = 'loading';
    errDetail = '';
    try {
      const { url } = await api.play(id);
      if (mine !== ticket || !video) return;
      if (!Hls.isSupported()) {
        video.src = url;
        await video.play();
        return;
      }
      const pre = BUFFER[ui.settings.buffer as keyof typeof BUFFER] ?? BUFFER.balanced;
      const h = new Hls({
        maxBufferLength: pre.ahead,
        maxMaxBufferLength: pre.ahead * 2,
        backBufferLength: 30,
        liveSyncDurationCount: pre.live,
        liveMaxLatencyDurationCount: pre.live + 6,
        playlistLoadPolicy: { default: { maxTimeToFirstByteMs: 10000, maxLoadTimeMs: 20000, timeoutRetry: { maxNumRetry: 4, retryDelayMs: 500, maxRetryDelayMs: 4000 }, errorRetry: { maxNumRetry: 6, retryDelayMs: 500, maxRetryDelayMs: 4000 } } },
      });
      let retries = 0;
      h.on(Hls.Events.MANIFEST_PARSED, () => video?.play().catch(() => {}));
      h.on(Hls.Events.FRAG_LOADED, () => (retries = 0));
      h.on(Hls.Events.ERROR, (_e, d) => {
        if (!d.fatal) return;
        if (mine !== ticket) return;
        if (d.details === Hls.ErrorDetails.MANIFEST_PARSING_ERROR || d.details === Hls.ErrorDetails.MANIFEST_LOAD_ERROR) {
          stop();
          startTs(url, mine);
          return;
        }
        if (d.type === Hls.ErrorTypes.NETWORK_ERROR && retries++ < 6) h.startLoad();
        else if (d.type === Hls.ErrorTypes.MEDIA_ERROR && retries++ < 2) h.recoverMediaError();
        else fail(`${d.type}: ${d.details}`);
      });
      hls = h;
      h.loadSource(url);
      h.attachMedia(video);
    } catch (e) {
      if (mine === ticket) fail(String(e));
    }
  }

  $effect(() => {
    const id = cur.id;
    if (video) start(id);
    // a stream that never starts should say so
    const wait = setTimeout(() => {
      if (status === 'loading') fail('timeout: no data from the provider within 25 s');
    }, 25_000);
    return () => clearTimeout(wait);
  });
  $effect(() => () => {
    ticket++;
    stop();
  });

  function zap(d: number) {
    const n = rp.list.length;
    rp.index = (rp.index + d + n) % n;
  }
  const close = () => {
    if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    ui.rplay = null;
  };
  const toggle = () => (video?.paused ? video.play().catch(() => {}) : video?.pause());
  const mute = () => {
    if (video) video.muted = !video.muted;
  };
  function fullscreen() {
    if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    else root?.requestFullscreen().catch(() => {});
  }

  function wake() {
    idle = false;
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => (idle = true), 3000);
  }
  $effect(() => {
    wake();
    return () => clearTimeout(idleTimer);
  });

  function onkey(e: KeyboardEvent) {
    wake();
    if ((e.target as HTMLElement)?.tagName === 'INPUT') return;
    const k = e.key;
    if (k === 'Escape') {
      if (!document.fullscreenElement) close();
    } else if (k === ' ' || k === 'k') toggle();
    else if (k === 'ArrowUp' || k === 'PageUp') zap(-1);
    else if (k === 'ArrowDown' || k === 'PageDown') zap(1);
    else if (k === 'm') mute();
    else if (k === 'f') fullscreen();
    else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onkey} />
<svelte:document onfullscreenchange={() => (full = !!document.fullscreenElement)} />

<div class="rp" class:idle bind:this={root} role="dialog" aria-label={cur.name} tabindex="-1" onmousemove={wake} ondblclick={fullscreen} data-overlay>
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={video}
    playsinline
    onplaying={() => { status = 'playing'; paused = false; }}
    onwaiting={() => { if (status === 'playing') status = 'buffering'; }}
    onpause={() => (paused = true)}
    onplay={() => (paused = false)}
    onvolumechange={() => (muted = !!video?.muted)}
    onclick={toggle}
  ></video>

  <div class="top">
    <button class="rb" onclick={close} aria-label={t('common.back')}><Icon name="back" size={22} /></button>
    <div class="who">
      <span class="mono">{cur.number ?? ''}</span>
      <b>{cur.name}</b>
      <span class="muted">{cur.group}</span>
    </div>
  </div>

  {#if status === 'loading' || status === 'buffering'}
    <div class="mid" role="status"><span class="spin"></span><span class="mono">{t('rp.buffering')}</span></div>
  {:else if status === 'error'}
    <div class="mid err" role="alert">
      <b>{t('rp.error')}</b>
      <p>{errKind === 'codec' ? t('rp.codec') : t('rp.other')}</p>
      <p class="mono muted">{errDetail}</p>
      {#if report}<pre class="mono muted rep">{report}</pre>{/if}
      <div class="ea">
        <button class="btn primary" onclick={() => start(cur.id)}>{t('rp.retry')}</button>
        <button class="btn" onclick={diagnose} disabled={diagBusy}>{t('rp.details')}</button>
        {#if rp.list.length > 1}<button class="btn" onclick={() => zap(1)}>{t('player.nextCh')}</button>{/if}
      </div>
    </div>
  {/if}

  <div class="bottom">
    <button class="rb" onclick={() => zap(-1)} aria-label={t('player.prevCh')}><Icon name="prev" size={22} /></button>
    <button class="rb big" onclick={toggle} aria-label={paused ? t('player.play') : t('player.pause')}><Icon name={paused ? 'play' : 'pause'} size={26} filled /></button>
    <button class="rb" onclick={() => zap(1)} aria-label={t('player.nextCh')}><Icon name="next" size={22} /></button>
    <span class="sp"></span>
    <button class="rb" onclick={mute} aria-label={muted ? t('player.unmute') : t('player.mute')} aria-pressed={muted}><Icon name="volume" size={22} /></button>
    <button class="rb" onclick={fullscreen} aria-label={t('player.fullscreen')} aria-pressed={full}><Icon name="expand" size={22} /></button>
  </div>
</div>

<style>
  .rp { position: fixed; inset: 0; z-index: 60; background: #000; color: #fff; display: grid; }
  .rp.idle { cursor: none; }
  video { width: 100%; height: 100%; object-fit: contain; background: #000; }
  .top, .bottom { position: absolute; inset-inline: 0; display: flex; align-items: center; gap: 10px; padding: 18px 24px; background: rgba(0, 0, 0, 0.55); transition: opacity 0.2s; }
  .top { top: 0; }
  .bottom { bottom: 0; }
  .rp.idle .top, .rp.idle .bottom { opacity: 0; pointer-events: none; }
  .who { display: flex; align-items: baseline; gap: 12px; min-width: 0; }
  .who b { font-size: 20px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who .muted { font-size: 13px; color: rgba(255, 255, 255, 0.65); }
  .rb { width: 48px; height: 48px; display: grid; place-items: center; border-radius: 50%; color: #fff; }
  .rb:hover { background: rgba(255, 255, 255, 0.16); }
  .rb.big { width: 56px; height: 56px; background: rgba(255, 255, 255, 0.14); }
  .sp { flex: 1; }
  .mid { position: absolute; inset: 0; margin: auto; width: fit-content; height: fit-content; max-width: min(520px, 90vw); display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; padding: 24px; }
  .mid.err { background: rgba(0, 0, 0, 0.78); border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 14px; }
  .mid.err b { font-size: 20px; }
  .mid p { margin: 0; line-height: 1.5; }
  .mid .muted { font-size: 12px; color: rgba(255, 255, 255, 0.55); overflow-wrap: anywhere; }
  .rep { text-align: start; white-space: pre-wrap; margin: 0; user-select: text; }
  .ea { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; }
  .spin { width: 34px; height: 34px; border-radius: 50%; border: 3px solid rgba(255, 255, 255, 0.25); border-top-color: #fff; animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 3s; } }
</style>

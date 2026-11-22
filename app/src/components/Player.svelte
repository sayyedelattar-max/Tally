<script lang="ts">
  import Art from './Art.svelte';
  import Icon from './Icon.svelte';
  import { ui, t, tune, stop, toast, toggleRecord, chName, progTitle } from '../lib/store.svelte';
  import { channelById, channels, clock, nextOn, nowOn, progress, titleById } from '../lib/mock';

  const p = $derived(ui.playing!);
  const live = $derived(p.kind === 'live' || p.kind === 'replay' ? channelById(p.id) : null);
  const item = $derived(p.kind === 'vod' ? titleById(p.id) : null);
  const ep = $derived(p.kind === 'vod' && item?.seasons ? item.seasons[p.season ?? 0]?.[p.ep ?? 0] : null);
  const prog = $derived(
    p.kind === 'live' ? nowOn(live!) : p.kind === 'replay' ? live!.progs.find((x) => x.start === p.start)! : null,
  );

  const heading = $derived(
    p.kind === 'live'
      ? chName(live!)
      : p.kind === 'replay'
        ? `${chName(live!)} · ${t('player.replay')}`
        : item!.kind === 'series'
          ? `${item!.title} · S${(p.season ?? 0) + 1} E${(p.ep ?? 0) + 1}`
          : item!.title,
  );
  const sub = $derived(
    p.kind === 'live' ? progTitle(prog!) : p.kind === 'replay' ? `${progTitle(prog!)} · ${clock(prog!.start)}` : ep ? ep.title : `${item!.year} · ${item!.cat}`,
  );
  const seed = $derived(live ? live.name : item!.title);
  const duration = $derived(p.kind === 'replay' ? (prog!.end - prog!.start) * 60 : p.kind === 'vod' ? (ep ? ep.mins : item!.mins ?? 90) * 60 : 0);
  const isLive = $derived(p.kind === 'live');

  // simulated playback position (0..1) for VOD / replay
  let pos = $state(0);
  let paused = $state(false);
  let muted = $state(false);
  $effect(() => {
    // restart position whenever a different thing starts playing
    const start = p.kind === 'vod' ? (p.season === undefined ? item!.progress ?? 0 : ep?.progress && ep.progress < 1 ? ep.progress : 0) : 0;
    pos = start;
    paused = false;
  });
  $effect(() => {
    if (isLive || paused || !duration) return;
    const id = setInterval(() => (pos = Math.min(1, pos + 1 / duration)), 1000);
    return () => clearInterval(id);
  });

  // chrome visibility: show on any activity, hide after a short idle
  let shown = $state(true);
  let hideTimer: ReturnType<typeof setTimeout>;
  function touch() {
    shown = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (!menu) shown = false;
    }, 3500);
  }
  $effect(() => {
    touch();
    return () => clearTimeout(hideTimer);
  });

  // channel banner after zapping
  let banner = $state(true);
  $effect(() => {
    void ui.channelId;
    banner = true;
    const id = setTimeout(() => (banner = false), 3200);
    return () => clearTimeout(id);
  });

  let menu = $state<null | 'tracks' | 'aspect'>(null);
  let aspect = $state<'auto' | '16:9' | '4:3' | 'fill'>('auto');
  let audio = $state('orig');
  let subs = $state('off');

  const zapList = $derived(channels.filter((c) => !ui.hidden.live.includes(c.cat)).toSorted((a, b) => a.num - b.num));
  function zap(d: number) {
    if (!isLive) return;
    const i = zapList.findIndex((c) => c.id === ui.channelId);
    const n = zapList[(i + d + zapList.length) % zapList.length];
    tune(n.id);
    ui.playing = { kind: 'live', id: n.id };
  }
  function lastChannel() {
    const id = ui.prevChannelId;
    tune(id);
    ui.playing = { kind: 'live', id };
  }
  function nextEpisode() {
    if (p.kind !== 'vod' || !item?.seasons) return;
    const s = p.season ?? 0;
    const e = (p.ep ?? 0) + 1;
    if (e < item.seasons[s].length) ui.playing = { kind: 'vod', id: item.id, season: s, ep: e };
    else if (s + 1 < item.seasons.length) ui.playing = { kind: 'vod', id: item.id, season: s + 1, ep: 0 };
  }
  const hasNext = $derived(p.kind === 'vod' && !!item?.seasons);

  // typing a channel number, like a TV remote
  let digits = $state('');
  let digitTimer: ReturnType<typeof setTimeout>;
  function commitDigits() {
    const n = Number(digits);
    digits = '';
    const c = channels.find((x) => x.num === n);
    if (c) {
      tune(c.id);
      ui.playing = { kind: 'live', id: c.id };
    } else toast(t('player.noChannel'));
  }

  function fmt(sec: number) {
    const s = Math.floor(sec);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    return `${h ? h + ':' : ''}${String(m).padStart(h ? 2 : 1, '0')}:${String(s % 60).padStart(2, '0')}`;
  }

  async function toggleFullscreen() {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch {
      toast(t('player.fsBlocked'));
    }
  }

  function onkey(e: KeyboardEvent) {
    touch();
    if (e.key === 'Escape') {
      e.preventDefault();
      if (menu) menu = null;
      else stop();
    } else if (e.key === ' ' && (e.target as HTMLElement).tagName !== 'BUTTON') {
      e.preventDefault();
      if (!isLive) paused = !paused;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      zap(1);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      zap(-1);
    } else if (e.key === 'ArrowRight' && !isLive) {
      e.preventDefault();
      pos = Math.min(1, pos + 10 / duration);
    } else if (e.key === 'ArrowLeft' && !isLive) {
      e.preventDefault();
      pos = Math.max(0, pos - 10 / duration);
    } else if (e.key.toLowerCase() === 'f') {
      toggleFullscreen();
    } else if (isLive && /^[0-9]$/.test(e.key)) {
      digits = (digits + e.key).slice(0, 3);
      clearTimeout(digitTimer);
      digitTimer = setTimeout(commitDigits, 1300);
    }
  }

  let root: HTMLDivElement;
  $effect(() => {
    root?.querySelector<HTMLElement>('[data-first]')?.focus();
  });
</script>

<svelte:window onkeydown={onkey} />

<div class="player" class:idle={!shown} data-overlay role="dialog" tabindex="-1" aria-label={heading} bind:this={root} onmousemove={touch} onpointermove={touch}>
  <div class="video" data-aspect={aspect}>
    <Art seed={seed} kind="wide" label={isLive ? progTitle(prog!) : item ? item.title : progTitle(prog!)} sub={live ? `CH ${live.num} · ${live.code}` : item!.cat} />
  </div>

  {#if paused && !isLive}<div class="paused" aria-hidden="true"><Icon name="pause" size={42} /></div>{/if}
  {#if digits}<div class="digits display" role="status">CH {digits}<span class="blink">_</span></div>{/if}

  {#if isLive && banner}
    {@const c = channelById(ui.channelId)}
    <div class="banner" role="status">
      <span class="num display">{c.num}</span>
      <div class="bt">
        <b>{chName(c)}</b>
        <span>{progTitle(nowOn(c))}</span>
        <span class="muted">{t('hero.next')}: <span class="mono ltr">{clock(nextOn(c).start)}</span> {progTitle(nextOn(c))}</span>
      </div>
    </div>
  {/if}

  <div class="pbar top">
    <button class="btn square" data-first onclick={stop} aria-label={t('common.back')}><Icon name="back" size={22} flip /></button>
    <div class="ti">
      <div class="h">
        {#if isLive}<span class="dot"></span>{/if}
        <b class="display">{heading}</b>
        {#if isLive}<span class="mono">{t('hero.liveNow')}</span>{/if}
        {#if isLive && ui.recording.includes(ui.channelId)}<span class="mono recb">REC</span>{/if}
      </div>
      <span class="s">{sub}</span>
    </div>
    <div class="grp">
      <button class="btn square" class:on={menu === 'aspect'} aria-label={t('player.aspect')} onclick={() => (menu = menu === 'aspect' ? null : 'aspect')}><span class="mono">{aspect === 'auto' ? 'A' : aspect}</span></button>
      <button class="btn square" class:on={menu === 'tracks'} aria-label={t('player.tracks')} onclick={() => (menu = menu === 'tracks' ? null : 'tracks')}><Icon name="cc" size={22} /></button>
      <button class="btn square" aria-label={t('player.fullscreen')} onclick={toggleFullscreen}><Icon name="expand" size={20} /></button>
    </div>
  </div>

  {#if menu === 'tracks'}
    <div class="menu" role="group" aria-label={t('player.tracks')}>
      <h3>{t('player.audio')}</h3>
      {#each [['orig', t('player.original')], ['ar', 'العربية'], ['en', 'English']] as [id, name]}
        <button class="opt" role="radio" aria-checked={audio === id} onclick={() => (audio = id)}>{#if audio === id}<Icon name="check" size={16} />{:else}<span class="sp"></span>{/if}{name}</button>
      {/each}
      <h3>{t('player.subtitles')}</h3>
      {#each [['off', t('common.off')], ['ar', 'العربية'], ['en', 'English']] as [id, name]}
        <button class="opt" role="radio" aria-checked={subs === id} onclick={() => (subs = id)}>{#if subs === id}<Icon name="check" size={16} />{:else}<span class="sp"></span>{/if}{name}</button>
      {/each}
    </div>
  {:else if menu === 'aspect'}
    <div class="menu" role="group" aria-label={t('player.aspect')}>
      <h3>{t('player.aspect')}</h3>
      {#each [['auto', t('player.auto')], ['16:9', '16:9'], ['4:3', '4:3'], ['fill', t('player.fill')]] as [id, name]}
        <button class="opt" role="radio" aria-checked={aspect === id} onclick={() => { aspect = id as typeof aspect; menu = null; }}>{#if aspect === id}<Icon name="check" size={16} />{:else}<span class="sp"></span>{/if}{name}</button>
      {/each}
    </div>
  {/if}

  <div class="pbar bottom">
    {#if isLive}
      <div class="live-ctl">
        <button class="btn square" aria-label={t('player.prevCh')} onclick={() => zap(-1)}><Icon name="prev" size={20} filled /></button>
        <button class="btn square" aria-label={t('player.nextCh')} onclick={() => zap(1)}><Icon name="next" size={20} filled /></button>
        <button class="btn" onclick={lastChannel}><Icon name="replay" size={18} /> {t('live.last')}</button>
        <button class="btn" class:on={ui.recording.includes(ui.channelId)} onclick={() => toggleRecord(ui.channelId)}><Icon name="rec" size={18} filled={ui.recording.includes(ui.channelId)} /> {ui.recording.includes(ui.channelId) ? t('live.stopRec') : t('live.record')}</button>
        <button class="btn square" aria-label={muted ? t('player.unmute') : t('player.mute')} onclick={() => (muted = !muted)}><Icon name="volume" size={20} /></button>
      </div>
      <div class="timeline">
        <span class="mono ltr">{clock(nowOn(channelById(ui.channelId)).start)}</span>
        <span class="bar"><i style="width:{progress(nowOn(channelById(ui.channelId))) * 100}%"></i></span>
        <span class="mono ltr">{clock(nowOn(channelById(ui.channelId)).end)}</span>
      </div>
    {:else}
      <div class="vod-ctl">
        <button class="btn square primary" aria-label={paused ? t('player.play') : t('player.pause')} onclick={() => (paused = !paused)}><Icon name={paused ? 'play' : 'pause'} size={22} filled={paused} /></button>
        {#if hasNext}<button class="btn square" aria-label={t('player.nextEp')} onclick={nextEpisode}><Icon name="next" size={20} filled /></button>{/if}
        <span class="mono ltr tm">{fmt(pos * duration)}</span>
        <input class="seek" type="range" min="0" max="1000" value={Math.round(pos * 1000)} oninput={(e) => (pos = Number((e.currentTarget as HTMLInputElement).value) / 1000)} aria-label={t('player.seek')} style="--pos:{pos * 100}%" />
        <span class="mono ltr tm">{fmt(duration)}</span>
        <button class="btn square" aria-label={muted ? t('player.unmute') : t('player.mute')} onclick={() => (muted = !muted)}><Icon name="volume" size={20} /></button>
      </div>
    {/if}
  </div>
</div>

<style>
  .player {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: #000;
    color: #f4f5f5;
    overflow: hidden;
  }
  .player.idle {
    cursor: none;
  }
  .player.idle .pbar,
  .player.idle .menu {
    opacity: 0;
    pointer-events: none;
  }
  .video {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  .video :global(.art) {
    width: min(100vw, calc(100dvh * 16 / 9));
  }
  .video[data-aspect='4:3'] :global(.art) {
    width: min(100vw, calc(100dvh * 4 / 3));
    aspect-ratio: 4 / 3;
  }
  .video[data-aspect='fill'] :global(.art) {
    width: max(100vw, calc(100dvh * 16 / 9));
  }
  .video :global(.label) {
    font-size: 6.5cqw !important;
  }
  .paused {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
    color: #fff;
  }
  .digits {
    position: absolute;
    top: 96px;
    inset-inline-end: 40px;
    padding: 6px 18px;
    background: rgba(8, 9, 10, 0.8);
    border-radius: 10px;
    font-size: 56px;
  }
  .blink {
    animation: blink 1s steps(2) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  .banner {
    position: absolute;
    inset-inline-end: 40px;
    top: 110px;
    display: flex;
    gap: 20px;
    align-items: center;
    padding: 18px 24px;
    background: rgba(8, 9, 10, 0.82);
    border-radius: 12px;
    max-width: 560px;
  }
  .banner .num {
    font-size: 64px;
    color: var(--accent);
  }
  .bt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .bt b {
    font-size: 20px;
  }
  .bt .muted {
    font-size: 13px;
  }
  .pbar.top,
  .pbar.bottom {
    position: absolute;
    inset-inline: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 28px;
    background: rgba(8, 9, 10, 0.78);
    transition: opacity 0.25s;
    height: auto;
    overflow: visible;
    border-radius: 0;
  }
  .pbar.top {
    top: 0;
  }
  .pbar.bottom {
    bottom: 0;
  }
  .ti {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .h {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .h b {
    font-size: 30px;
    text-transform: none;
    font-weight: 600;
  }
  .s {
    color: #c3c7cb;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .recb {
    background: var(--live);
    padding: 2px 8px;
    border-radius: 4px;
    color: #fff;
  }
  .grp {
    display: flex;
    gap: 8px;
  }
  .player :global(.btn) {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.28);
  }
  .player :global(.btn:hover) {
    background: rgba(255, 255, 255, 0.16);
  }
  .player :global(.btn.on) {
    background: rgba(255, 255, 255, 0.22);
    border-color: #fff;
  }
  .player :global(.btn.primary) {
    background: var(--accent);
    border-color: var(--accent);
  }
  .menu {
    position: absolute;
    top: 84px;
    inset-inline-end: 28px;
    width: 260px;
    padding: 12px;
    background: rgba(18, 20, 22, 0.97);
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 12px;
    transition: opacity 0.25s;
  }
  .menu h3 {
    margin: 8px 8px 4px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 400;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #9aa1a9;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 42px;
    padding: 0 10px;
    border-radius: 8px;
  }
  .opt:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .sp {
    width: 16px;
  }
  .live-ctl {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .timeline {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 14px;
    color: #c3c7cb;
  }
  .timeline .bar {
    flex: 1;
    position: static;
    padding: 0;
    background: rgba(255, 255, 255, 0.25);
    height: 4px;
    color: #fff;
    display: block;
  }
  .vod-ctl {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
  }
  .tm {
    color: #c3c7cb;
    min-width: 54px;
    text-align: center;
  }
  .seek {
    flex: 1;
    min-width: 0;
    appearance: none;
    height: 28px;
    background: transparent;
    cursor: pointer;
    direction: ltr;
  }
  .seek::-webkit-slider-runnable-track {
    height: 5px;
    border-radius: 3px;
    background: linear-gradient(to right, #fff var(--pos), rgba(255, 255, 255, 0.28) var(--pos));
  }
  .seek::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    margin-top: -5.5px;
    border-radius: 50%;
    background: #fff;
    border: 0;
  }
  .seek:focus-visible {
    outline-offset: 6px;
  }
  @media (max-width: 760px) {
    .h b {
      font-size: 22px;
    }
    .banner {
      inset-inline-end: 12px;
      top: 90px;
    }
    .pbar.top,
    .pbar.bottom {
      padding: 12px 14px;
    }
  }
</style>

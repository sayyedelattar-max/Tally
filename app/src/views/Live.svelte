<script lang="ts">
  import Art from '../components/Art.svelte';
  import Icon from '../components/Icon.svelte';
  import { ui, t, tune, play, toggleFav, toggleRecord, chName, progTitle, toast } from '../lib/store.svelte';
  import { artFor } from '../lib/art';
  import { normalize, matches } from '../lib/norm';
  import { CLOCK0, channelById, channels, clock, liveCategories, nextOn, nowOn, progress } from '../lib/mock';
  import type { Channel, Prog } from '../lib/types';

  type Mode = 'channels' | 'guide' | 'multi';
  let mode = $state<Mode>('channels');
  let cat = $state('all');
  let q = $state('');
  let sel = $state<{ ch: number; p: Prog } | null>(null);
  let reminders = $state<string[]>([]);
  let scheduled = $state<string[]>([]);

  const cats = $derived(liveCategories.filter((c) => !ui.hidden.live.includes(c)));
  const visible = $derived(
    channels.filter(
      (c) => !ui.hidden.live.includes(c.cat) && (cat === 'all' || c.cat === cat) && matches(normalize(`${c.name} ${c.nameAr ?? ''} ${c.num}`), normalize(q)),
    ),
  );
  const ch = $derived(channelById(ui.channelId));
  const now = $derived(nowOn(ch));
  const nx = $derived(nextOn(ch));
  const upcoming = $derived(ch.progs.filter((p) => p.start >= now.end).slice(0, 4));
  const isFav = $derived(ui.favs.includes(ch.id));
  const isRec = $derived(ui.recording.includes(ch.id));

  function pickRow(c: Channel) {
    if (mode === 'multi') {
      ui.multi[ui.multiFocus] = c.id;
      toast(`${t('live.tile')} ${ui.multiFocus + 1}: ${chName(c)}`);
    } else tune(c.id);
  }

  // ----- guide window: 4 hours starting 30 minutes ago
  const W0 = -30;
  const WLEN = 240;
  const ticks = $derived.by(() => {
    const first = Math.ceil((CLOCK0 + W0) / 30) * 30 - CLOCK0;
    const out: number[] = [];
    for (let m = first; m < W0 + WLEN; m += 30) out.push(m);
    return out;
  });
  const pct = (m: number) => ((m - W0) / WLEN) * 100;
  const blocks = (c: Channel) =>
    c.progs
      .filter((p) => p.end > W0 && p.start < W0 + WLEN)
      .map((p) => ({ p, left: pct(Math.max(p.start, W0)), width: pct(Math.min(p.end, W0 + WLEN)) - pct(Math.max(p.start, W0)) }));

  const keyOf = (id: number, p: Prog) => `${id}-${p.start}`;
  function toggleIn(list: string[], k: string, on: string, off: string) {
    const i = list.indexOf(k);
    if (i >= 0) {
      list.splice(i, 1);
      toast(off);
    } else {
      list.push(k);
      toast(on);
    }
  }
</script>

<div class="tools">
  <div class="seg" role="group" aria-label={t('live.mode')}>
    <button class:on={mode === 'channels'} onclick={() => (mode = 'channels')}><Icon name="tv" size={16} /> {t('live.channels')}</button>
    <button class:on={mode === 'guide'} onclick={() => (mode = 'guide')}><Icon name="guide" size={16} /> {t('live.guide')}</button>
    <button class:on={mode === 'multi'} onclick={() => (mode = 'multi')}><Icon name="grid" size={16} /> {t('live.multiview')}</button>
  </div>
  <label class="field find">
    <Icon name="search" size={16} />
    <input bind:value={q} placeholder={t('live.filter')} aria-label={t('live.filter')} />
  </label>
</div>
<div class="chips" role="group" aria-label={t('browse.categories')}>
  <button class="chip" class:on={cat === 'all'} onclick={() => (cat = 'all')}>{t('common.all')}</button>
  {#each cats as c}
    <button class="chip" class:on={cat === c} onclick={() => (cat = c)}>{t('cat.' + c)}</button>
  {/each}
</div>

{#if mode === 'guide'}
  <div class="detailbar">
    {#if sel}
      {@const c = channelById(sel.ch)}
      {@const k = keyOf(c.id, sel.p)}
      {@const state = sel.p.start <= 0 && sel.p.end > 0 ? 'now' : sel.p.start > 0 ? 'future' : 'past'}
      <div class="di">
        <span class="tag muted">{state === 'now' ? t('guide.onNow') : state === 'future' ? `${t('home.in')} ${sel.p.start} ${t('common.min')}` : t('guide.ended')}</span>
        <b class="dt">{progTitle(sel.p)}</b>
        <span class="mono muted ltr">CH {c.num} · {chName(c)} · {clock(sel.p.start)} – {clock(sel.p.end)}</span>
      </div>
      <div class="da">
        {#if state === 'now'}
          <button class="btn primary" onclick={() => play({ kind: 'live', id: c.id })}><Icon name="play" filled size={16} /> {t('hero.watchLive')}</button>
        {:else if state === 'future'}
          <button class="btn" class:on={reminders.includes(k)} onclick={() => toggleIn(reminders, k, t('toast.remOn'), t('toast.remOff'))}>{reminders.includes(k) ? t('guide.reminderSet') : t('guide.remind')}</button>
          <button class="btn" class:on={scheduled.includes(k)} onclick={() => toggleIn(scheduled, k, t('toast.recScheduled'), t('toast.recCancelled'))}><Icon name="rec" size={16} filled={scheduled.includes(k)} /> {t('live.record')}</button>
        {:else if c.catchup}
          <button class="btn primary" onclick={() => play({ kind: 'replay', id: c.id, start: sel!.p.start })}><Icon name="replay" size={16} /> {t('guide.watchReplay')}</button>
        {:else}
          <span class="muted">{t('guide.noCatchup')}</span>
        {/if}
      </div>
    {:else}
      <span class="muted">{t('guide.pick')}</span>
    {/if}
  </div>

  <div class="gridwrap">
    <div class="gridin">
      <div class="grow head">
        <div class="glabel"></div>
        <div class="lane">
          {#each ticks as m}<span class="tick mono ltr" style="inset-inline-start:{pct(m)}%">{clock(m)}</span>{/each}
          <span class="nowline" style="inset-inline-start:{pct(0)}%"></span>
        </div>
      </div>
      {#each visible as c (c.id)}
        <div class="grow">
          <button class="glabel" onclick={() => play({ kind: 'live', id: c.id })}>
            <span class="mono muted num">{c.num}</span>
            <span class="gname">{chName(c)}</span>
          </button>
          <div class="lane">
            {#each blocks(c) as b}
              <button
                class="blk"
                class:cur={b.p.start <= 0 && b.p.end > 0}
                class:picked={sel && sel.ch === c.id && sel.p.start === b.p.start}
                style="inset-inline-start:{b.left}%;width:{b.width}%"
                onclick={() => (sel = { ch: c.id, p: b.p })}
              >
                <span class="bt">{progTitle(b.p)}</span>
                <span class="mono muted ltr">{clock(b.p.start)}{#if scheduled.includes(keyOf(c.id, b.p))} · REC{/if}{#if reminders.includes(keyOf(c.id, b.p))} · ◔{/if}</span>
              </button>
            {/each}
            <span class="nowline" style="inset-inline-start:{pct(0)}%"></span>
          </div>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class="split">
    <div class="list" role="list" aria-label={t('live.channels')}>
      {#each visible as c (c.id)}
        {@const a = artFor(c.name)}
        {@const n = nowOn(c)}
        <div class="lrow" class:cur={c.id === ui.channelId} role="listitem">
          <button class="pick" onclick={() => pickRow(c)}>
            <span class="mono muted num">{c.num}</span>
            <span class="plate" style="background:{a.bg};color:{a.ink}">{c.code}</span>
            <span class="lt">
              <b>{chName(c)}</b>
              <span class="muted">{progTitle(n)}</span>
              <span class="bar"><i style="width:{progress(n) * 100}%"></i></span>
            </span>
          </button>
          <button class="fav" class:on={ui.favs.includes(c.id)} aria-pressed={ui.favs.includes(c.id)} aria-label={t('live.favorite')} onclick={() => toggleFav(c.id)}>
            <Icon name="star" size={18} filled={ui.favs.includes(c.id)} />
          </button>
        </div>
      {:else}
        <p class="empty">{t('browse.none')}</p>
      {/each}
    </div>

    {#if mode === 'channels'}
      <div class="stage">
        <button class="screen" onclick={() => play({ kind: 'live', id: ch.id })} aria-label={t('live.fullscreen')}>
          <Art seed={ch.name} kind="wide" label={progTitle(now)} sub={`CH ${ch.num} · ${ch.code}`} />
          <span class="badge"><span class="dot"></span><span class="mono">{t('hero.liveNow')}</span></span>
          {#if isRec}<span class="badge rec mono">REC</span>{/if}
          <span class="bar scr"><i style="width:{progress(now) * 100}%"></i></span>
        </button>
        <div class="meta">
          <div class="mm">
            <h2 class="display">{chName(ch)}</h2>
            <span class="mono muted ltr">{clock(now.start)} – {clock(now.end)}</span>
          </div>
          <p class="nowt">{progTitle(now)}</p>
          <p class="muted">{t('hero.next')}: <span class="mono ltr">{clock(nx.start)}</span> · {progTitle(nx)}</p>
        </div>
        <div class="acts">
          <button class="btn primary" onclick={() => play({ kind: 'live', id: ch.id })}><Icon name="expand" size={18} /> {t('live.fullscreen')}</button>
          <button class="btn" class:on={isFav} onclick={() => toggleFav(ch.id)}><Icon name="star" size={18} filled={isFav} /> {t('live.favorite')}</button>
          <button class="btn" class:on={isRec} onclick={() => toggleRecord(ch.id)}><Icon name="rec" size={18} filled={isRec} /> {isRec ? t('live.stopRec') : t('live.record')}</button>
          <button class="btn" onclick={() => tune(ui.prevChannelId)}><Icon name="replay" size={18} /> {t('live.last')}</button>
        </div>
        <ul class="up">
          {#each upcoming as p}
            <li><span class="mono ltr tm">{clock(p.start)}</span><span>{progTitle(p)}</span><span class="mono muted ltr">{p.end - p.start} {t('common.min')}</span></li>
          {/each}
        </ul>
      </div>
    {:else}
      <div class="stage">
        <p class="muted hint">{t('live.multiHint')}</p>
        <div class="mgrid">
          {#each ui.multi as id, i}
            {@const mc = channelById(id)}
            <button class="tile" class:aud={i === ui.multiFocus} aria-pressed={i === ui.multiFocus} onclick={() => (ui.multiFocus = i)}>
              <Art seed={mc.name} kind="wide" label={progTitle(nowOn(mc))} sub={`CH ${mc.num} · ${mc.code}`} />
              <span class="audio mono">{i === ui.multiFocus ? t('live.audio') : t('live.muted')}</span>
            </button>
          {/each}
        </div>
        <div class="acts">
          <button class="btn primary" onclick={() => play({ kind: 'live', id: ui.multi[ui.multiFocus] })}><Icon name="expand" size={18} /> {t('live.fullscreen')} · {t('live.tile')} {ui.multiFocus + 1}</button>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .tools {
    display: flex;
    gap: 14px;
    align-items: center;
    flex-wrap: wrap;
    margin: 8px 0 6px;
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .find {
    flex: 1;
    max-width: 340px;
    margin-inline-start: auto;
  }
  .split {
    display: grid;
    grid-template-columns: minmax(300px, 400px) minmax(0, 1fr);
    gap: 28px;
    align-items: start;
    margin-top: 6px;
  }
  .list {
    border: 1px solid var(--line);
    border-radius: 12px;
    overflow: hidden;
    max-height: calc(100dvh - 250px);
    overflow-y: auto;
    background: var(--surface);
  }
  .lrow {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--line);
    position: relative;
  }
  .lrow:last-child {
    border-bottom: 0;
  }
  .lrow.cur {
    background: var(--surface-3);
  }
  .lrow.cur::before {
    content: '';
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    width: 3px;
    background: var(--accent);
  }
  .pick {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 6px 10px 14px;
    min-height: 68px;
  }
  .pick:hover {
    background: var(--surface-2);
  }
  .num {
    width: 30px;
    flex: none;
  }
  .plate {
    flex: none;
    width: 46px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 15px;
    letter-spacing: 0.03em;
  }
  .lt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .lt b {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lt .muted {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lt .bar {
    margin-top: 4px;
    color: var(--accent);
  }
  .fav {
    width: 48px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .fav:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .fav.on {
    color: var(--accent);
  }

  .stage {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .screen {
    position: relative;
    display: block;
    border-radius: 14px;
    overflow: hidden;
    width: 100%;
  }
  .screen:hover {
    outline: 2px solid var(--line-2);
  }
  .badge {
    position: absolute;
    top: 14px;
    inset-inline-start: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    background: rgba(10, 11, 13, 0.72);
    border-radius: 6px;
    color: #fff;
  }
  .badge.rec {
    inset-inline-start: auto;
    inset-inline-end: 14px;
    background: var(--live);
    font-weight: 500;
  }
  .scr {
    position: absolute;
    inset-inline: 0;
    bottom: 0;
    height: 4px;
    border-radius: 0;
    color: #fff;
  }
  .mm {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .mm h2 {
    font-size: 40px;
  }
  .nowt {
    font-size: 19px;
    font-weight: 600;
    margin-top: 2px;
  }
  .acts {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .up {
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 1px solid var(--line);
  }
  .up li {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    gap: 16px;
    padding: 11px 4px;
    border-bottom: 1px solid var(--line);
  }
  .tm {
    color: var(--accent);
  }
  .hint {
    font-size: 14px;
  }
  .mgrid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .tile {
    position: relative;
    display: block;
    border-radius: 10px;
    overflow: hidden;
    outline: 3px solid transparent;
    outline-offset: -3px;
  }
  .tile.aud {
    outline-color: var(--accent);
  }
  .audio {
    position: absolute;
    top: 10px;
    inset-inline-end: 10px;
    padding: 4px 8px;
    background: rgba(10, 11, 13, 0.75);
    border-radius: 5px;
    color: #fff;
  }
  .tile.aud .audio {
    background: var(--accent);
    color: var(--on-accent);
  }

  /* guide */
  .detailbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    min-height: 88px;
    margin: 6px 0 14px;
    padding: 14px 18px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
  }
  .di {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .dt {
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 600;
    line-height: 1.1;
  }
  .da {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .gridwrap {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
  }
  .gridin {
    min-width: 900px;
  }
  .grow {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    border-bottom: 1px solid var(--line);
  }
  .grow:last-child {
    border-bottom: 0;
  }
  .glabel {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    min-height: 60px;
    border-inline-end: 1px solid var(--line);
  }
  button.glabel:hover {
    background: var(--surface-2);
  }
  .gname {
    font-weight: 600;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lane {
    position: relative;
    min-height: 60px;
    direction: ltr;
  }
  .grow.head .lane {
    min-height: 36px;
  }
  .grow.head .glabel {
    min-height: 36px;
  }
  .tick {
    position: absolute;
    top: 10px;
    padding-inline-start: 6px;
    border-inline-start: 1px solid var(--line);
    color: var(--muted);
    height: 26px;
  }
  .nowline {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent);
    pointer-events: none;
    z-index: 2;
  }
  .blk {
    position: absolute;
    top: 4px;
    bottom: 4px;
    padding: 6px 9px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    overflow: hidden;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    text-align: start;
  }
  .blk:hover {
    background: var(--surface-3);
  }
  .blk.cur {
    border-color: var(--muted);
  }
  .blk.picked {
    border-color: var(--accent);
    background: var(--surface-3);
  }
  .bt {
    font-weight: 500;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .blk .mono {
    font-size: 11px;
  }
  @media (max-width: 900px) {
    .split {
      grid-template-columns: 1fr;
    }
    .list {
      max-height: 320px;
    }
  }
</style>

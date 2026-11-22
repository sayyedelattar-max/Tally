<script lang="ts">
  import Art from '../components/Art.svelte';
  import Icon from '../components/Icon.svelte';
  import Row from '../components/Row.svelte';
  import TitleCard from '../components/TitleCard.svelte';
  import { ui, t, go, play, tune, openTitle, chName, progTitle } from '../lib/store.svelte';
  import { artFor } from '../lib/art';
  import { CLOCK0, TODAY, channelById, channels, clock, fmtDuration, movies, nowOn, nextOn, progress, series, startingSoon, titleById } from '../lib/mock';
  import type { Prog } from '../lib/types';

  // ---- time-aware greeting
  const hour = TODAY.getHours();
  const part = hour < 5 ? 'night' : hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';

  // ---- resume: the channel you were on
  const cur = $derived(channelById(ui.channelId));
  const curNow = $derived(nowOn(cur));
  const curNext = $derived(nextOn(cur));
  const spec = $derived(artFor(cur.name));

  // ---- launcher tiles
  const tiles = $derived([
    { view: 'live' as const, icon: 'tv', label: t('nav.live'), n: channels.filter((c) => !ui.hidden.live.includes(c.cat)).length },
    { view: 'movies' as const, icon: 'film', label: t('nav.movies'), n: movies.length },
    { view: 'series' as const, icon: 'layers', label: t('nav.series'), n: series.length },
    { view: 'replay' as const, icon: 'replay', label: t('nav.replay'), n: channels.filter((c) => c.catchup).length },
  ]);

  // ---- "on your channels": a live timetable
  const FROM = -30;
  const SPAN = 210;
  const lanes = $derived(ui.favs.map((id) => channelById(id)).filter((c) => !ui.hidden.live.includes(c.cat)).slice(0, 6));
  const pct = (m: number) => ((Math.min(Math.max(m, FROM), FROM + SPAN) - FROM) / SPAN) * 100;
  const blocks = (c: (typeof channels)[number]) => c.progs.filter((p) => p.end > FROM && p.start < FROM + SPAN);
  const ticks = Array.from({ length: Math.floor(SPAN / 30) + 1 }, (_, i) => FROM + i * 30).filter((m) => (CLOCK0 + m) % 30 === 0 && m > FROM);
  function pickBlock(c: (typeof channels)[number], p: Prog) {
    if (p.start <= 0 && p.end > 0) play({ kind: 'live', id: c.id });
    else {
      tune(c.id);
      go('live');
    }
  }

  // ---- "something that fits"
  let fit = $state<30 | 60 | 120 | 0>(120);
  const fitOpts = [
    { v: 30 as const, k: 'home.fit30' },
    { v: 60 as const, k: 'home.fit60' },
    { v: 120 as const, k: 'home.fit120' },
    { v: 0 as const, k: 'home.fitAny' },
  ];
  const fits = $derived.by(() => {
    const live = startingSoon(25, 8)
      .filter((s) => !ui.hidden.live.includes(s.ch.cat) && (fit === 0 || s.p.end - s.p.start <= fit))
      .slice(0, 3);
    const films = movies.filter((m) => !ui.hidden.movies.includes(m.cat) && (fit === 0 || (m.mins ?? 0) <= fit)).slice(0, 8);
    return { live, films };
  });

  const cont = $derived([titleById('m5'), titleById('s3'), titleById('m1'), titleById('s1')]);
</script>

<section class="top">
  <div class="greet">
    <h2 class="display">{t('home.greet.' + part)}</h2>
    <p class="muted">{t('home.lead')}</p>
  </div>
</section>

<section class="deck" aria-label={t('home.resume')}>
  <div class="resume" style="--bg-c:{spec.bg};--ink:{spec.ink}">
    <div class="rart"><Art seed={cur.name} kind="wide" /></div>
    <div class="rcopy">
      <span class="mono tagline"><span class="dot"></span> {t('home.resume')} · CH {cur.num}</span>
      <h3 class="display">{chName(cur)}</h3>
      <p class="np">{progTitle(curNow)}</p>
      <div class="bar"><i style="width:{progress(curNow) * 100}%"></i></div>
      <p class="mono small ltr">{clock(curNow.start)} – {clock(curNow.end)} <span class="nx">· {t('hero.next')} {clock(curNext.start)} {progTitle(curNext)}</span></p>
      <div class="acts">
        <button class="hbtn solid" onclick={() => play({ kind: 'live', id: cur.id })}><Icon name="play" filled size={18} /> {t('home.resumeBtn')}</button>
        <button class="hbtn" onclick={() => { tune(ui.prevChannelId); play({ kind: 'live', id: ui.channelId }); }}><Icon name="replay" size={18} /> {t('live.last')}</button>
      </div>
    </div>
  </div>

  <nav class="tiles" aria-label={t('home.browse')}>
    {#each tiles as tl}
      <button class="tile" onclick={() => go(tl.view)}>
        <Icon name={tl.icon} size={28} />
        <span class="tn display">{tl.label}</span>
        <span class="mono muted tc">{tl.n}</span>
      </button>
    {/each}
  </nav>
</section>

<section class="tt" aria-label={t('home.timeline')}>
  <div class="row-head"><h2>{t('home.timeline')}</h2><button class="more muted" onclick={() => go('live')}>{t('live.guide')} <Icon name="chevron" size={16} flip /></button></div>
  <div class="tscroll">
    <div class="tgrid">
      <div class="axis" aria-hidden="true">
        <span class="lbl"></span>
        <div class="track">
          <span class="nowlbl mono" style="inset-inline-start:{pct(0)}%">{t('home.now')}</span>
          {#each ticks as m}<span class="tick mono ltr" style="inset-inline-start:{pct(m)}%">{clock(m)}</span>{/each}
        </div>
      </div>
      {#each lanes as c (c.id)}
        {@const a = artFor(c.name)}
        <div class="lane">
          <button class="lbl" onclick={() => play({ kind: 'live', id: c.id })} aria-label={chName(c)}>
            <span class="plate" style="background:{a.bg};color:{a.ink}">{c.code}</span>
            <span class="ln">{chName(c)}</span>
          </button>
          <div class="track">
            {#each ticks as m}<i class="grid" style="inset-inline-start:{pct(m)}%"></i>{/each}
            {#each blocks(c) as p}
              {@const on = p.start <= 0 && p.end > 0}
              <button
                class="blk"
                class:on
                class:past={p.end <= 0}
                style="inset-inline-start:{pct(p.start)}%;width:calc({pct(p.end) - pct(p.start)}% - 3px)"
                onclick={() => pickBlock(c, p)}
                title="{progTitle(p)} · {clock(p.start)}"
              >
                <b>{progTitle(p)}</b>
                <span class="mono ltr">{clock(p.start)}</span>
                {#if on}<i class="fill" style="width:{progress(p) * 100}%"></i>{/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
      <div class="playhead" style="inset-inline-start:calc(var(--lbl) + (100% - var(--lbl)) * {pct(0) / 100})" aria-hidden="true"></div>
    </div>
  </div>
</section>

<section class="fit">
  <div class="fh">
    <h2>{t('home.fitTitle')}</h2>
    <div class="seg" role="group" aria-label={t('home.fitTitle')}>
      {#each fitOpts as o}
        <button class:on={fit === o.v} onclick={() => (fit = o.v)}>{t(o.k)}</button>
      {/each}
    </div>
  </div>
  <div class="scroller">
    {#each fits.live as s}
      <button class="fcard live" onclick={() => { tune(s.ch.id); go('live'); }}>
        <span class="mono tagline"><span class="dot"></span> {t('home.in')} {s.p.start} {t('common.min')}</span>
        <b class="display">{progTitle(s.p)}</b>
        <span class="muted">{chName(s.ch)} · {s.p.end - s.p.start} {t('common.min')}</span>
      </button>
    {/each}
    {#each fits.films as m (m.id)}
      {@const a = artFor(m.title)}
      <button class="fcard" style="--bg-c:{a.bg};--ink:{a.ink}" onclick={() => openTitle(m)}>
        <span class="mono tagline">{t('hero.movie')} · {fmtDuration(m.mins ?? 0, ui.lang)}</span>
        <b class="display">{m.title}</b>
        <span class="muted">{m.cat} · {m.year}</span>
      </button>
    {/each}
    {#if !fits.live.length && !fits.films.length}<p class="empty">{t('home.fitNone')}</p>{/if}
  </div>
</section>

<Row title={t('home.continue')}>
  {#each cont as item (item.id)}<TitleCard {item} wide />{/each}
</Row>

<style>
  .top { padding: 6px 0 18px; }
  .greet h2 { font-size: clamp(40px, 4.6vw, 64px); text-transform: none; font-weight: 600; letter-spacing: 0; }
  .greet p { margin: 8px 0 0; }

  .deck {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr);
    gap: 20px;
  }
  .resume {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
    min-height: 330px;
    border-radius: 18px;
    overflow: hidden;
    background: var(--bg-c);
    color: var(--ink);
  }
  .rcopy { display: flex; flex-direction: column; gap: 10px; justify-content: center; padding: 30px 32px; align-items: flex-start; min-width: 0; z-index: 1; }
  .tagline { display: inline-flex; align-items: center; gap: 8px; opacity: 0.9; }
  .rcopy h3 { font-size: clamp(34px, 3.6vw, 54px); }
  .np { margin: 0; font-size: 18px; font-weight: 500; }
  .rcopy .bar { width: min(100%, 320px); color: var(--ink); }
  .small { font-size: 12px; opacity: 0.88; max-width: 38ch; }
  .nx { display: block; margin-top: 2px; }
  .acts { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 8px; }
  .rart { position: absolute; inset: 0; inset-inline-start: 46%; }
  .rart :global(*) { max-width: none; }

  .hbtn { display: inline-flex; align-items: center; gap: 8px; min-height: 48px; padding: 0 20px; border-radius: 10px; font-weight: 600; border: 1.5px solid currentColor; color: var(--ink); }
  .hbtn.solid { background: var(--ink); color: var(--bg-c); border-color: var(--ink); }

  .tiles { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .tile {
    display: flex; flex-direction: column; align-items: flex-start; justify-content: space-between; gap: 18px;
    padding: 20px; min-height: 150px; border: 1px solid var(--line-2); border-radius: 16px; background: var(--surface);
    text-align: start; transition: background 0.12s, border-color 0.12s;
  }
  .tile:hover { background: var(--surface-3); border-color: var(--muted); }
  .tn { font-size: 30px; }
  .tc { margin-top: -14px; }

  section.tt, section.fit { margin-top: 34px; }
  .more { display: inline-flex; align-items: center; gap: 4px; font-size: 14px; padding: 6px 10px; border-radius: 8px; }
  .more:hover { color: var(--text); background: var(--surface-2); }

  .tscroll { overflow-x: auto; padding-bottom: 6px; }
  .tgrid { --lbl: 168px; position: relative; min-width: 760px; display: flex; flex-direction: column; border: 1px solid var(--line); border-radius: 14px; background: var(--surface); }
  .axis, .lane { display: grid; grid-template-columns: var(--lbl) minmax(0, 1fr); }
  .axis { height: 34px; border-bottom: 1px solid var(--line); }
  .lane + .lane { border-top: 1px solid var(--line); }
  .track { position: relative; min-height: 60px; }
  .axis .track { min-height: 0; }
  .tick, .nowlbl { position: absolute; top: 9px; transform: translateX(-50%); color: var(--muted); font-size: 11px; white-space: nowrap; }
  :global([dir='rtl']) .tick, :global([dir='rtl']) .nowlbl { transform: translateX(50%); }
  .nowlbl { color: var(--live); font-weight: 600; }
  .grid { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--line); }
  .lbl { display: flex; align-items: center; gap: 10px; padding: 0 14px; text-align: start; min-width: 0; }
  button.lbl:hover { background: var(--surface-3); }
  .plate { display: grid; place-items: center; width: 40px; height: 28px; border-radius: 6px; font: 700 12px/1 var(--font-mono); flex: none; }
  .ln { font-weight: 600; font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .blk {
    position: absolute; top: 6px; bottom: 6px; display: flex; flex-direction: column; justify-content: center; gap: 1px;
    padding: 0 10px; border-radius: 8px; background: var(--surface-3); border: 1px solid var(--line-2);
    text-align: start; overflow: hidden; min-width: 0; z-index: 1;
  }
  .blk b { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .blk span { font-size: 11px; color: var(--muted); }
  .blk:hover { border-color: var(--text); }
  .blk.past { opacity: 0.45; }
  .blk.on { background: var(--text); color: var(--bg); border-color: var(--text); }
  .blk.on span { color: var(--bg); opacity: 0.7; }
  .fill { position: absolute; inset-inline-start: 0; bottom: 0; height: 3px; background: var(--live); }
  .playhead { position: absolute; top: 0; bottom: 0; width: 2px; background: var(--live); pointer-events: none; z-index: 0; }

  .fh { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; margin-bottom: 14px; }
  .fh h2 { margin: 0; }
  .fcard {
    flex: none; width: 250px; min-height: 150px; display: flex; flex-direction: column; justify-content: space-between; gap: 10px;
    padding: 18px; border-radius: 14px; text-align: start; background: var(--bg-c, var(--surface)); color: var(--ink, var(--text));
    border: 1px solid var(--line-2);
  }
  .fcard :global(.muted), .fcard.live .muted { color: inherit; opacity: 0.8; }
  .fcard b { font-size: 26px; line-height: 1.05; text-wrap: balance; }
  .fcard.live { border-color: var(--live); }
  .fcard:hover { outline: 2px solid var(--accent); outline-offset: 2px; }

  @media (max-width: 1100px) {
    .deck { grid-template-columns: 1fr; }
    .tiles { grid-template-columns: repeat(4, 1fr); }
    .tile { min-height: 110px; padding: 14px; }
    .tn { font-size: 22px; }
  }
  @media (max-width: 760px) {
    .resume { grid-template-columns: 1fr; }
    .rart { inset-inline-start: 0; opacity: 0.35; }
    .tiles { grid-template-columns: 1fr 1fr; }
    .tgrid { --lbl: 120px; min-width: 680px; }
  }
</style>

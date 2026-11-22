<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { ui, t, play, chName, progTitle, locale } from '../lib/store.svelte';
  import { artFor } from '../lib/art';
  import { channels, clock, dayDate, dayIndex, dayStart } from '../lib/mock';

  const list = $derived(channels.filter((c) => c.catchup && !ui.hidden.live.includes(c.cat)));
  let chId = $state(201);
  let day = $state(0);

  const ch = $derived(list.find((c) => c.id === chId) ?? list[0]);
  const rows = $derived(
    ch.progs
      .filter((p) => dayIndex(p.start) === day && p.start < 0)
      .toReversed(),
  );
  const days = [0, -1, -2, -3];
  const dayLabel = (d: number) =>
    d === 0 ? t('replay.today') : d === -1 ? t('replay.yesterday') : dayDate(d).toLocaleDateString(locale(), { weekday: 'long' });
  const dateLabel = (d: number) => dayDate(d).toLocaleDateString(locale(), { day: 'numeric', month: 'short' });
  const ongoing = (p: { start: number; end: number }) => p.start <= 0 && p.end > 0;
</script>

<p class="muted lead">{t('replay.lead')}</p>

<div class="split">
  <div class="list" role="list" aria-label={t('replay.channels')}>
    {#each list as c (c.id)}
      {@const a = artFor(c.name)}
      <button class="lrow" class:cur={c.id === ch.id} onclick={() => (chId = c.id)}>
        <span class="plate" style="background:{a.bg};color:{a.ink}">{c.code}</span>
        <span class="lt"><b>{chName(c)}</b><span class="mono muted">CH {c.num} · 3 {t('replay.daysBack')}</span></span>
      </button>
    {/each}
  </div>

  <div class="right">
    <div class="chips" role="group" aria-label={t('replay.day')}>
      {#each days as d}
        <button class="chip" class:on={day === d} onclick={() => (day = d)}>{dayLabel(d)} <span class="mono">{dateLabel(d)}</span></button>
      {/each}
    </div>
    <ul class="progs">
      {#each rows as p (p.start)}
        <li>
          <button class="prow" onclick={() => play({ kind: 'replay', id: ch.id, start: p.start })}>
            <span class="mono ltr tm">{clock(p.start)}</span>
            <span class="pt">
              <b>{progTitle(p)}</b>
              <span class="mono muted ltr">{clock(p.start)} – {clock(p.end)} · {p.end - p.start} {t('common.min')}</span>
            </span>
            {#if ongoing(p)}<span class="pill liveb">{t('hero.liveNow')}</span>{/if}
            <span class="go"><Icon name={ongoing(p) ? 'play' : 'replay'} size={20} /></span>
          </button>
        </li>
      {:else}
        <li class="empty">{t('browse.none')}</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .lead {
    margin: 10px 0 16px;
    max-width: 70ch;
  }
  .split {
    display: grid;
    grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
    gap: 28px;
    align-items: start;
  }
  .list {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    overflow-y: auto;
    max-height: calc(100dvh - 230px);
  }
  .lrow {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 62px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
    position: relative;
  }
  .lrow:last-child {
    border-bottom: 0;
  }
  .lrow:hover {
    background: var(--surface-2);
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
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .lt b {
    font-weight: 600;
  }
  .progs {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
  }
  .prow {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto 32px;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 62px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
  }
  .prow:hover {
    background: var(--surface);
  }
  .tm {
    color: var(--accent);
  }
  .pt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .pt b {
    font-weight: 600;
  }
  .liveb {
    color: var(--live);
  }
  .go {
    color: var(--muted);
  }
  @media (max-width: 900px) {
    .split {
      grid-template-columns: 1fr;
    }
    .list {
      max-height: 240px;
    }
  }
</style>

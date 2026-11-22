<script lang="ts">
  import { artFor, shapes } from '../lib/art';

  let { seed, kind = 'poster', label = '', sub = '' }: { seed: string; kind?: 'poster' | 'wide'; label?: string; sub?: string } = $props();

  const W = $derived(kind === 'poster' ? 200 : 320);
  const H = $derived(kind === 'poster' ? 300 : 180);
  const spec = $derived(artFor(seed));
  const list = $derived(shapes(spec.comp, W, H, spec.a, spec.b, spec.r));
</script>

<div class="art {kind}" style="background:{spec.bg};color:{spec.ink}">
  <svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
    {#each list as s}
      {#if s.t === 'circle'}
        <circle cx={s.cx} cy={s.cy} r={s.r} fill={s.fill ?? 'none'} stroke={s.stroke ?? 'none'} stroke-width={s.sw ?? 0} />
      {:else if s.t === 'rect'}
        <rect x={s.x} y={s.y} width={s.w} height={s.h} fill={s.fill} />
      {:else}
        <path d={s.d} fill={s.fill} />
      {/if}
    {/each}
  </svg>
  {#if label}<span class="label">{label}</span>{/if}
  {#if sub}<span class="sub">{sub}</span>{/if}
</div>

<style>
  .art {
    position: relative;
    overflow: hidden;
    container-type: inline-size;
    width: 100%;
  }
  .poster {
    aspect-ratio: 2 / 3;
  }
  .wide {
    aspect-ratio: 16 / 9;
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .label {
    position: absolute;
    inset-inline: 7cqw;
    bottom: 7cqw;
    font-family: var(--font-display);
    font-weight: 700;
    text-transform: uppercase;
    line-height: 1;
    letter-spacing: 0.01em;
    text-wrap: balance;
    overflow-wrap: anywhere;
  }
  .poster .label {
    font-size: 15cqw;
  }
  .wide .label {
    font-size: 9cqw;
  }
  .sub {
    position: absolute;
    inset-inline-start: 7cqw;
    top: 7cqw;
    font-family: var(--font-mono);
    font-size: 4.6cqw;
    letter-spacing: 0.04em;
    opacity: 0.85;
  }
</style>

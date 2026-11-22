<script lang="ts">
  import Art from './Art.svelte';
  import { play, chName, progTitle } from '../lib/store.svelte';
  import { nowOn, progress } from '../lib/mock';
  import type { Channel } from '../lib/types';

  let { ch, width = 260 }: { ch: Channel; width?: number } = $props();
  const now = $derived(nowOn(ch));
</script>

<button class="card" style="width:{width}px" onclick={() => play({ kind: 'live', id: ch.id })}>
  <div class="frame">
    <Art seed={ch.name} kind="wide" label={ch.code} sub={`CH ${ch.num}`} />
    <span class="live"><span class="dot"></span></span>
    <div class="bar"><i style="width:{progress(now) * 100}%"></i></div>
  </div>
  <div class="cap">
    <b>{chName(ch)}</b>
    <span>{progTitle(now)}</span>
  </div>
</button>

<style>
  .live {
    position: absolute;
    top: 10px;
    inset-inline-end: 10px;
    display: flex;
    padding: 5px;
    background: rgba(10, 11, 13, 0.7);
    border-radius: 999px;
  }
</style>

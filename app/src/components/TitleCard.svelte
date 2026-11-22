<script lang="ts">
  import Art from './Art.svelte';
  import { t, openTitle, ui } from '../lib/store.svelte';
  import { fmtDuration } from '../lib/mock';
  import type { Title } from '../lib/types';

  let { item, wide = false, width = 176 }: { item: Title; wide?: boolean; width?: number } = $props();

  const meta = $derived(
    item.kind === 'movie'
      ? `${item.year} · ${item.cat} · ${fmtDuration(item.mins ?? 0, ui.lang)}`
      : `${item.year} · ${item.cat} · ${item.seasons?.length} ${t('detail.seasons')}`,
  );
</script>

<button class="card" style="width:{wide ? 280 : width}px" onclick={() => openTitle(item)}>
  <div class="frame">
    <Art seed={item.title} kind={wide ? 'wide' : 'poster'} label={item.title} />
    {#if item.progress !== undefined}<div class="bar"><i style="width:{item.progress * 100}%"></i></div>{/if}
  </div>
  <div class="cap">
    <b>{item.title}</b>
    <span>{meta}</span>
  </div>
</button>

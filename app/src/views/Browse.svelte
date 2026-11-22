<script lang="ts">
  import TitleCard from '../components/TitleCard.svelte';
  import Detail from './Detail.svelte';
  import { ui, t } from '../lib/store.svelte';
  import { movies, series, titleById } from '../lib/mock';

  let { kind }: { kind: 'movie' | 'series' } = $props();

  let cat = $state('all');
  let sort = $state<'new' | 'az'>('new');

  const all = $derived(kind === 'movie' ? movies : series);
  const hidden = $derived(kind === 'movie' ? ui.hidden.movies : ui.hidden.series);
  const cats = $derived([...new Set(all.map((x) => x.cat))].filter((c) => !hidden.includes(c)));
  const list = $derived(
    all
      .filter((x) => !hidden.includes(x.cat) && (cat === 'all' || x.cat === cat))
      .toSorted((a, b) => (sort === 'az' ? a.title.localeCompare(b.title) : a.added - b.added)),
  );
  const open = $derived(ui.detail ? titleById(ui.detail) : null);
</script>

{#if open && open.kind === kind}
  <Detail item={open} />
{:else}
  <div class="bar-tools">
    <div class="chips" role="group" aria-label={t('browse.categories')}>
      <button class="chip" class:on={cat === 'all'} onclick={() => (cat = 'all')}>{t('common.all')} <span class="mono">{all.filter((x) => !hidden.includes(x.cat)).length}</span></button>
      {#each cats as c}
        <button class="chip" class:on={cat === c} onclick={() => (cat = c)}>{c}</button>
      {/each}
    </div>
    <div class="seg" role="group" aria-label={t('browse.sort')}>
      <button class:on={sort === 'new'} onclick={() => (sort = 'new')}>{t('browse.recent')}</button>
      <button class:on={sort === 'az'} onclick={() => (sort = 'az')}>A–Z</button>
    </div>
  </div>

  {#if list.length}
    <div class="grid">
      {#each list as item (item.id)}<TitleCard {item} width={200} />{/each}
    </div>
  {:else}
    <p class="empty">{t('browse.none')}</p>
  {/if}
{/if}

<style>
  .bar-tools {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin: 8px 0 18px;
  }
  .bar-tools .chips {
    flex: 1;
    min-width: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    gap: 26px 18px;
    padding-top: 6px;
  }
  .grid :global(.card) {
    width: auto !important;
  }
</style>

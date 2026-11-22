<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/store.svelte';

  let { title, onmore, children }: { title: string; onmore?: () => void; children: Snippet } = $props();
</script>

<section>
  <div class="row-head">
    <h2>{title}</h2>
    {#if onmore}
      <button class="more muted" onclick={onmore}>{t('common.seeAll')} <Icon name="chevron" size={16} flip /></button>
    {/if}
  </div>
  <div class="scroller">{@render children()}</div>
</section>

<style>
  .more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 14px;
    padding: 6px 10px;
    border-radius: 8px;
  }
  .more:hover {
    color: var(--text);
    background: var(--surface-2);
  }
</style>

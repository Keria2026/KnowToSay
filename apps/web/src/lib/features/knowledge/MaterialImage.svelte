<script lang="ts">
  import { onMount } from 'svelte';
  import { Image as ImageIcon } from 'lucide-svelte';
  import { materialUrl } from '../workspace/api';
  import type { Material } from '$lib/types';
  let { material }: { material: Material } = $props();
  let container: HTMLSpanElement;
  let visible = $state(false);
  let url = $state('');
  let failed = $state(false);
  onMount(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          visible = true;
          observer.disconnect();
        }
      },
      { rootMargin: '160px' },
    );
    observer.observe(container);
    return () => observer.disconnect();
  });
  $effect(() => {
    const current = material;
    let active = true;
    url = '';
    failed = false;
    if (visible && current.hash)
      materialUrl(current)
        .then((value) => {
          if (active) url = value;
        })
        .catch(() => {
          if (active) failed = true;
        });
    return () => {
      active = false;
    };
  });
</script>

<span class="material-image" bind:this={container}>
  {#if url && !failed}<img
      src={url}
      alt={material.name}
      loading="lazy"
      decoding="async"
      onerror={() => (failed = true)}
    />
  {:else}<ImageIcon size={30} strokeWidth={1.25} />{#if failed}<span
        class="field-help">图片无法预览</span
      >{/if}{/if}
</span>

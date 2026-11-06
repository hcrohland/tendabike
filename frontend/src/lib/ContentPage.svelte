<script lang="ts">
  import { marked } from "marked";
  import { getLocale } from "../../paraglide/runtime";
  import { resolveContent } from "./content";

  let { page }: { page: string } = $props();

  let html = $state("");
  let loaded = $state(false);

  $effect(() => {
    const locale = getLocale();
    resolveContent(page, locale).then((markdown) => {
      html = markdown ? (marked.parse(markdown) as string) : "";
      loaded = true;
    });
  });
</script>

{#if loaded && !html}
  <div class="p-4 sm:p-6 md:p-8 text-gray-500 dark:text-gray-400">
    Content not found.
  </div>
{:else if html}
  <div class="prose dark:prose-invert max-w-none p-4 sm:p-6 md:p-8">
    {@html html}
  </div>
{/if}

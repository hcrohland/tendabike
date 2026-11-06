<script lang="ts">
  import { marked } from "marked";
  import { getLocale } from "../../paraglide/runtime";
  import { resolveContent } from "./content";

  let { page }: { page: string } = $props();

  let html = $state("");

  $effect(() => {
    const locale = getLocale();
    resolveContent(page, locale).then((markdown) => {
      html = marked.parse(markdown) as string;
    });
  });
</script>

{#if html}
  <div class="prose dark:prose-invert max-w-none p-4 sm:p-6 md:p-8">
    {@html html}
  </div>
{/if}

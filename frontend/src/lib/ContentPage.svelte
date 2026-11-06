<script lang="ts">
  import { marked } from "marked";
  import { getLocale } from "../../paraglide/runtime";
  import { resolveContent } from "./content";

  let { page }: { page: string } = $props();

  const html = $derived(
    marked.parse(resolveContent(page, getLocale())) as string,
  );
</script>

<div class="prose dark:prose-invert max-w-none p-4 sm:p-6 md:p-8">
  {@html html}
</div>

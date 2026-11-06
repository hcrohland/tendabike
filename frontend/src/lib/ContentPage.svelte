<script lang="ts">
  import { marked } from "marked";
  import { getLocale } from "../../paraglide/runtime";
  import { resolveContent } from "./content";

  let { page }: { page: string } = $props();

  const html = $derived(() => {
    const locale = getLocale();
    const markdown = resolveContent(page, locale);
    return marked.parse(markdown) as string;
  });
</script>

<div class="prose max-w-none p-4 sm:p-6 md:p-8">
  {@html html()}
</div>

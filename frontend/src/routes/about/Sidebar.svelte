<script lang="ts">
  import { page } from "$app/state";
  import { topics } from "./[page]/topics";

  const path = $derived(page.url.pathname);
  const isOverview = $derived(path === "/about");
  const current = $derived(path.replace(/^\/about\//, ""));
</script>

<nav class="shrink-0 text-sm md:w-44">
  <ul class="space-y-1">
    <li>
      <a
        href="/about"
        class="block rounded px-2 py-1 {isOverview
          ? 'bg-gray-100 dark:bg-gray-800 font-semibold text-primary-600 dark:text-primary-400'
          : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
      >
        Overview
      </a>
    </li>
    {#each topics as topic (topic.slug)}
      <li>
        <a
          href="/about/{topic.slug}"
          class="block rounded px-2 py-1 {current === topic.slug
            ? 'bg-gray-100 dark:bg-gray-800 font-semibold text-primary-600 dark:text-primary-400'
            : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
        >
          {topic.label}
        </a>
      </li>
    {/each}
  </ul>
</nav>

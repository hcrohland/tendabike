<script lang="ts">
  import * as m from "../paraglide/messages";
</script>

<footer class="border-t border-gray-200 dark:border-gray-700 py-4 px-6 mt-8">
  <div class="flex items-center justify-between text-sm">
    <nav class="flex gap-4 text-gray-500 dark:text-gray-400">
      <a href="/about" class="hover:underline">{m.footer_about()}</a>
      <a href="/data-protection" class="hover:underline"
        >{m.footer_data_protection()}</a
      >
    </nav>
    <p class="text-gray-400 dark:text-gray-500">
      © Christoph Rohland 2019-2026 -
      <a
        href="https://github.com/hcrohland/tendabike"
        target="_blank"
        rel="noopener"
        class="hover:underline">Source Code</a
      >
    </p>
  </div>
</footer>

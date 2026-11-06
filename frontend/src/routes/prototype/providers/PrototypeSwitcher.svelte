<!-- PROTOTYPE (throwaway): floating variant switcher. Dev-only; hidden in
     production builds so a stray merge can't ship it. -->
<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";

  let {
    variants,
    names,
  }: { variants: string[]; names: Record<string, string> } = $props();

  const isDev = import.meta.env.DEV;
  let current = $derived(page.url.searchParams.get("variant") ?? variants[0]);
  let idx = $derived(Math.max(0, variants.indexOf(current)));

  function go(to: string) {
    const url = new URL(page.url.href);
    url.searchParams.set("variant", to);
    goto(url.toString(), { replaceState: true });
  }

  function cycle(dir: 1 | -1) {
    const next = variants[(idx + dir + variants.length) % variants.length];
    go(next);
  }

  $effect(() => {
    if (!isDev) return;
    function onKey(e: KeyboardEvent) {
      const el = document.activeElement as HTMLElement | null;
      if (
        el &&
        (el.tagName === "INPUT" ||
          el.tagName === "TEXTAREA" ||
          el.isContentEditable)
      )
        return;
      if (e.key === "ArrowRight") cycle(1);
      else if (e.key === "ArrowLeft") cycle(-1);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

{#if isDev}
  <div
    class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 rounded-full bg-neutral-900 text-white px-3 py-2 shadow-lg ring-1 ring-white/20"
  >
    <button
      class="rounded-full px-2 py-1 hover:bg-white/20"
      onclick={() => cycle(-1)}
      aria-label="Previous variant"
    >
      ←
    </button>
    <span class="text-sm font-medium whitespace-nowrap">
      {current} <span class="opacity-70">({names[current] ?? ""})</span>
    </span>
    <button
      class="rounded-full px-2 py-1 hover:bg-white/20"
      onclick={() => cycle(1)}
      aria-label="Next variant"
    >
      →
    </button>
  </div>
{/if}

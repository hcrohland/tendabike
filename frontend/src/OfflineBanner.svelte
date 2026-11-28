<script lang="ts">
  import { isOffline } from "./lib/provider.svelte";
  import { colorClasses, mainText } from "./Widgets/color.js";
  import * as m from "../paraglide/messages";
</script>

<!-- The offline banner: a successful /api/providers fetch
     that returned an empty enabled set. Plain text, not dismissible, no
     link. A failed fetch is a distinct error state and shows nothing here. -->
{#if isOffline()}
  <p
    class={colorClasses["secondary"].surface +
      " border-b px-4 py-2 text-sm " +
      mainText}
  >
    {m.offline_banner()}
  </p>
{/if}

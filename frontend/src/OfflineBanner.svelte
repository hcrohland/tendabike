<script lang="ts">
  import { isOffline } from "./lib/provider";
  import * as m from "../paraglide/messages";
</script>

<!-- The offline banner: a successful /api/providers fetch
     that returned an empty enabled set. Plain text, not dismissible, no
     link. A failed fetch is a distinct error state and shows nothing here. -->
{#if isOffline()}
  <p
    class="bg-surface-2 border-b border-border-subtle px-4 py-2 text-sm text-text-1"
  >
    {m.offline_banner()}
  </p>
{/if}

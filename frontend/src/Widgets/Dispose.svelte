<script lang="ts">
  import Switch from "../Widgets/Switch.svelte";
  import { m } from "../../paraglide/messages";

  let { dispose = $bindable(), name }: { dispose: boolean; name: string } =
    $props();
</script>

<Switch bind:checked={dispose}>
  {#if dispose}
    {m.dispose_active({ name })}
  {:else}
    {m.dispose_question({ name })}?
  {/if}
</Switch>

<script lang="ts">
  import Switch from "../Widgets/Switch.svelte";
  import { m } from "../../paraglide/messages";

  export let dispose: boolean;
  export let name: string;
</script>

<Switch bind:checked={dispose}>
  {#if dispose}
    {m.dispose_active({ name })}
  {:else}
    {m.dispose_question({ name })}?
  {/if}
</Switch>

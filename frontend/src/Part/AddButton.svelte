<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { actions } from "../Widgets/Actions.svelte";
  import type { Part } from "../lib/part";

  export let part: Part;
  export let tab: number | string;
  export let here: string;
  let action: () => void;
  let condition: boolean;

  $: if (here == "parts" && tab == here && part.isGear()) {
    action = () => $actions.installPart(part);
    condition = true;
  } else if (here == "plans" && tab == here) {
    action = () => $actions.newPlan(part);
    condition = true;
  } else if (here == "services" && tab == here) {
    action = () => $actions.newService(part);
    condition = true;
  } else {
    condition = false;
  }
</script>

{#if condition}
  <Button size="sm" color="light" on:click={action}>add</Button>
{/if}

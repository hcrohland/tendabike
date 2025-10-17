<script lang="ts">
  import { Modal } from "flowbite-svelte";
  import { types } from "../lib/types";
  import AttachForm from "./AttachForm.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";

  let part: Part | undefined = $state();
  let open = $state(false);
  let time = $state<Date>();
  let gear = $state<number>();
  let hook = $state<number>();

  async function onaction() {
    await part!.attach(time!, true, gear!, hook!);
    open = false;
  }

  export const start = (p: Part) => {
    part = p;
    open = true;
  };
</script>

{#if part}
  <Modal form bind:open {onaction} classes={{ body: "min-h-90" }}>
    {#snippet header()}
      Attach {types[part!.what].name}
      {part!.name}
      {part!.vendor}
      {part!.model}
    {/snippet}
    <AttachForm bind:time bind:gear bind:hook {part} />

    {#snippet footer()}
      <Buttons bind:open label={"Attach"} />
    {/snippet}
  </Modal>
{/if}

<script lang="ts">
  import Modal from "../Widgets/Modal.svelte";
  import NewForm from "./PartForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import { user } from "../lib/store";

  let type = $state<Type>();
  let part = $state<any>();
  let open = $state(false);

  async function onaction() {
    await new Part(part).create();
    open = false;
  }

  export function start(t: Type) {
    type = t;
    part = { ...new Part({ owner: $user?.id, what: t.id }) };
    open = true;
  }
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    New {type!.name}
  {/snippet}
  <NewForm {type} bind:part />
  {#snippet footer()}
    <Buttons bind:open label="Create" />
  {/snippet}
</Modal>

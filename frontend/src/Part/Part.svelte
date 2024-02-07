<script lang="ts">
  import { ButtonGroup, Button } from "@sveltestrap/sveltestrap";
  import { parts } from "../lib/store";
  import InstallPart from "../Actions/InstallPart.svelte";
  import ChangePart from "../Actions/ChangePart.svelte";
  import RecoverPart from "../Actions/RecoverPart.svelte";
  import AttachPart from "../Actions/AttachPart.svelte";
  import Subparts from "./Subparts.svelte";
  import GearCard from "./GearCard.svelte";
  import PartHist from "./PartHist.svelte";
  import { Part } from "../lib/types";
  import NewService from "../Service/NewService.svelte";
  import ServiceList from "../Service/ServiceList.svelte";

  export let params: { id: number; what: number };

  let installPart: (p: Part) => void;
  let changePart: (p: Part) => void;
  let newService: (p: Part) => void;
  let recoverPart: (p: Part) => void;
  let attachPart: (p: Part) => void;

  $: part = $parts[params.id];
  $: hook = part.type();
</script>

<GearCard {part} display>
  <ButtonGroup class="float-end">
    {#if part.disposed_at}
      <Button on:click={() => recoverPart(part)}>Recover gear</Button>
    {:else}
      <Button color="light" on:click={() => newService(part)}>
        Log Service
      </Button>
      {#if part.what == hook.main}
        <Button color="light" on:click={() => installPart(part)}>
          Install new part
        </Button>
      {:else}
        <Button color="light" on:click={() => attachPart(part)}>
          Attach part
        </Button>
      {/if}
      <Button color="light" on:click={() => changePart(part)}>
        Change details
      </Button>
    {/if}
  </ButtonGroup>
</GearCard>
<br />
<ServiceList {part} /><br />
<PartHist id={params.id} />
<Subparts gear={part} {hook} />

<AttachPart bind:attachPart />
<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />
<NewService bind:newService />

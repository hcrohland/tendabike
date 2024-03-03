<script lang="ts">
  import {
    ButtonGroup,
    Button,
    Dropdown,
    DropdownToggle,
    DropdownMenu,
    DropdownItem,
  } from "@sveltestrap/sveltestrap";
  import InstallPart from "../Attachment/InstallPart.svelte";
  import ChangePart from "./ChangePart.svelte";
  import RecoverPart from "./RecoverPart.svelte";
  import AttachPart from "../Attachment/AttachPart.svelte";
  import Subparts from "./Subparts.svelte";
  import GearCard from "./GearCard.svelte";
  import PartHist from "./PartHist.svelte";
  import NewService from "../Service/NewService.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { parts, Part } from "./part";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import ServiceList from "../Service/ServiceList.svelte";

  export let params: { id: number; what: number };

  let installPart: (p: Part) => void;
  let changePart: (p: Part) => void;
  let newService: (p: Part) => void;
  let recoverPart: (p: Part) => void;
  let attachPart: (p: Part) => void;
  let newPlan: (p: Part) => void;

  $: part = $parts[params.id];
  $: hook = part.type();
</script>

<GearCard {part} display>
  <ButtonGroup class="float-end">
    {#if part.disposed_at}
      <Button color="light" on:click={() => recoverPart(part)}
        >Recover gear</Button
      >
    {:else}
      <Button color="light" on:click={() => newService(part)}>
        Log Service
      </Button>
      <Dropdown direction="down">
        <DropdownToggle color="light" caret split />
        <DropdownMenu>
          <DropdownItem on:click={() => newPlan(part)}>
            New Service Plan
          </DropdownItem>
          {#if part.isGear()}
            <DropdownItem on:click={() => installPart(part)}>
              New Part
            </DropdownItem>
            <DropdownItem divider />
          {:else}
            <DropdownItem on:click={() => attachPart(part)}>
              Attach part
            </DropdownItem>
          {/if}
          <DropdownItem on:click={() => changePart(part)}>
            Change details
          </DropdownItem>
        </DropdownMenu>
      </Dropdown>
    {/if}
  </ButtonGroup>
</GearCard>
<br />
<PlanList {part} /><br />
<ServiceList {part} /><br />
<PartHist id={params.id} />
<Subparts gear={part} {hook} />

<AttachPart bind:attachPart />
<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />
<NewService bind:newService />
<NewPlan bind:newPlan />

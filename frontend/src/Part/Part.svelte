<script lang="ts">
  import {
    ButtonGroup,
    Button,
    Dropdown,
    DropdownToggle,
    DropdownMenu,
    DropdownItem,
    Badge,
    TabPane,
    TabContent,
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
  import { parts, Part } from "../lib/part";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import ServiceList from "../Service/ServiceList.svelte";
  import {
    plans,
    alerts_for_plans,
    plans_by_partid,
    plans_for_gear,
  } from "../lib/serviceplan";
  import { attachees_for_gear, attachments } from "../lib/attachment";
  import { services } from "../lib/service";
  import { usages } from "../lib/usage";
  import { filterValues } from "../lib/mapable";
  import Wizard from "./Wizard.svelte";

  export let params: { id: number; what: number };

  let installPart: (p: Part) => void;
  let changePart: (p: Part) => void;
  let newService: (p: Part) => void;
  let recoverPart: (p: Part) => void;
  let attachPart: (p: Part) => void;
  let newPlan: (p: Part) => void;

  $: part = $parts[params.id];
  $: hook = part.type();
  $: attachees = attachees_for_gear(part.id, $attachments);
  $: planlist = attachees.reduce(
    (list, att) => list.concat(plans_by_partid(att.part_id, $plans)),
    plans_for_gear(part.id, $plans, $attachments),
  );
  $: alerts = alerts_for_plans(
    planlist,
    $parts,
    $services,
    $usages,
    $attachments,
  );
</script>

<GearCard {part} display>
  <ButtonGroup class="float-end">
    {#if part.disposed_at}
      <Button color="light" on:click={() => recoverPart(part)}
        >Recover gear</Button
      >
    {:else}
      <Button color="light" on:click={() => changePart(part)}>
        Change details
      </Button>
      <Dropdown direction="down">
        <DropdownToggle color="light" caret split />
        <DropdownMenu>
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
          <DropdownItem on:click={() => newService(part)}>
            Log Service
          </DropdownItem>
          <DropdownItem on:click={() => newPlan(part)}>
            New Service Plan
          </DropdownItem>
        </DropdownMenu>
      </Dropdown>
    {/if}
  </ButtonGroup>
</GearCard>
<br />
<PartHist id={params.id} />
<TabContent>
  <TabPane tabId="parts" active>
    <strong slot="tab">
      Attached Parts
      {#if part.isGear()}
        <Button size="sm" color="light" on:click={() => installPart(part)}>
          add
        </Button>
      {/if}
    </strong>
    <Subparts {part} {hook} />
    {#if part.isGear()}
      <Wizard gear={part} {attachees} />
    {/if}
  </TabPane>
  <TabPane tabId="plans">
    <strong slot="tab">
      Service Plans
      {#if alerts.alert > 0}
        <Badge color="danger">{alerts.alert}</Badge>
      {:else if alerts.warn > 0}
        <Badge color="warning">{alerts.warn}</Badge>
      {/if}
      <Button size="sm" color="light" on:click={() => newPlan(part)}>
        add
      </Button> &NonBreakingSpace;
    </strong>
    <PlanList {planlist} part_id={part.id} /><br />
  </TabPane>
  <TabPane tabId="service">
    <strong slot="tab">
      Service Logs
      <Button size="sm" color="light" on:click={() => newService(part)}>
        add
      </Button>
    </strong>
    <ServiceList {part} /><br />
  </TabPane>
</TabContent>
<AttachPart bind:attachPart />
<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />
<NewService bind:newService />
<NewPlan bind:newPlan />

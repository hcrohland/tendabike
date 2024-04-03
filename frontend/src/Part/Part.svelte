<script lang="ts">
  import {
    ButtonGroup,
    Button,
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
    plans_for_part_and_attachees,
  } from "../lib/serviceplan";
  import { attachments } from "../lib/attachment";
  import { services } from "../lib/service";
  import { usages } from "../lib/usage";

  export let params: { id: number; what: number };

  let installPart: (p: Part) => void;
  let changePart: (p: Part) => void;
  let newService: (p: Part) => void;
  let recoverPart: (p: Part) => void;
  let attachPart: (p: Part) => void;
  let newPlan: (p: Part) => void;

  let tab: number | string = "parts";

  $: part = $parts[params.id];
  $: hook = part.type();
  $: planlist = plans_for_part_and_attachees($attachments, $plans, part.id);
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
      <Button color="light" on:click={() => recoverPart(part)}>
        Recover gear
      </Button>
    {:else}
      <Button color="light" on:click={() => changePart(part)}>
        Change details
      </Button>
      {#if !part.isGear()}
        <Button color="light" on:click={() => attachPart(part)}>
          Attach part
        </Button>
      {/if}
    {/if}
  </ButtonGroup>
</GearCard>
<br />
<PartHist id={params.id} />
<TabContent on:tab={(e) => (tab = e.detail)}>
  <TabPane tabId="parts" active>
    <strong slot="tab">
      Attached Parts
      {#if tab == "parts" && part.isGear()}
        <Button size="sm" color="light" on:click={() => installPart(part)}>
          add
        </Button>
      {/if}
    </strong>
    <Subparts {part} {hook} />
  </TabPane>
  <TabPane tabId="plans">
    <strong slot="tab">
      Service Plans
      {#if alerts.alert > 0}
        <Badge color="danger">{alerts.alert}</Badge>
      {:else if alerts.warn > 0}
        <Badge color="warning">{alerts.warn}</Badge>
      {/if}
      {#if tab == "plans"}
        <Button size="sm" color="light" on:click={() => newPlan(part)}>
          add
        </Button> &NonBreakingSpace;
      {/if}
    </strong>
    <PlanList {planlist} part_id={part.id} /><br />
  </TabPane>
  <TabPane tabId="service">
    <strong slot="tab">
      Service Logs
      {#if tab == "service"}
        <Button size="sm" color="light" on:click={() => newService(part)}>
          add
        </Button>
      {/if}
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

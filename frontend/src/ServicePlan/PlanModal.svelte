<script lang="ts">
  import { Button, Input, Label, Modal } from "flowbite-svelte";
  import { Limits, ServicePlan } from "../lib/serviceplan";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import GearForm from "../Widgets/GearForm.svelte";
  import type { Type } from "../lib/types";
  import type { Snippet } from "svelte";
  import PlanLimits from "./PlanLimits.svelte";

  interface Props {
    safePlan: (p: ServicePlan) => void;
    open: boolean;
    plan: ServicePlan;
    no_gear?: boolean;
    children?: Snippet;
  }

  let {
    safePlan,
    open = $bindable(),
    plan,
    no_gear = true,
    children,
  }: Props = $props();

  const sethook = (type: Type, h: number | null) => {
    what = type.id;
    hook = h;
  };

  function onaction() {
    let newplan = new ServicePlan({
      ...limits,
      id,
      part: gear,
      what,
      name,
      hook,
    });
    safePlan(newplan);
  }

  let { id, part: gear, what, name, hook } = $derived(plan);
  let limits: any = $state(plan.to_object());
</script>

<Modal size="xs" bind:open form {onaction}>
  {#snippet header()}
    {@render children?.()}
  {/snippet}
  {#if !no_gear}
    <TypeForm with_body onChange={sethook} />
    <Label>of</Label>
    <GearForm bind:gear />
  {/if}
  <Input
    type="text"
    id="inputName"
    bind:value={name}
    autofocus
    required
    placeholder="Name"
  />
  <PlanLimits bind:select={limits} />
  {#snippet footer()}
    <Button onclick={() => (open = false)} color="alternative">Cancel</Button>
    <Button
      type="submit"
      value="create"
      disabled={!Limits.valid(limits)}
      class="float-end"
    >
      Safe
    </Button>
  {/snippet}
</Modal>

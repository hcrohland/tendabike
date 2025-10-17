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
    no_gear?: boolean;
    children?: Snippet;
  }

  let { safePlan, no_gear = true, children }: Props = $props();

  let open = $state(false);
  const sethook = (type: Type, h: number | undefined) => {
    what = type.id;
    hook = h as number | null;
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
    open = false;
  }

  let plan = $state(new ServicePlan({}));
  let { id, part: gear, what, name, hook } = $derived(plan);
  let limits = $state({});

  export function start(p: ServicePlan) {
    plan = p;
    limits = p.to_object();
    open = true;
  }
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

<script lang="ts">
  import { ButtonGroup, Input, InputAddon } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import GearForm from "../Widgets/GearForm.svelte";
  import type { Type } from "../lib/types";
  import type { Snippet } from "svelte";
  import PlanLimits from "./PlanLimits.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

  interface Props {
    safePlan: (p: ServicePlan) => void;
    no_gear: boolean;
    children?: Snippet;
  }

  let { safePlan, no_gear, children }: Props = $props();

  let open = $state(false);
  let part = $state(null as number | null);
  let name = $state("");
  let limits = $state({});

  let what: number | null;
  let hook: number | null;
  let id: string | undefined;

  const sethook = (type: Type, h: number | undefined) => {
    what = type.id;
    hook = h as number | null;
  };

  function onaction() {
    let newplan = new ServicePlan({
      ...limits,
      id,
      part,
      what,
      name,
      hook,
    });
    safePlan(newplan);
    open = false;
  }

  export function start(p: ServicePlan) {
    id = p.id;
    part = p.part;
    name = p.name;
    what = p.what;
    hook = p.hook;
    limits = p.to_object();
    open = true;
  }
</script>

<Modal size="xs" bind:open {onaction}>
  {#snippet header()}
    {@render children?.()}
  {/snippet}
  {#if !no_gear}
    <ButtonGroup>
      <TypeForm
        with_body
        onChange={sethook}
        classes={{ select: "rounded-r-none h-full" }}
      />
      <InputAddon>of</InputAddon>
      <GearForm bind:gear={part} />
    </ButtonGroup>
  {/if}
  <Input type="text" bind:value={name} autofocus required placeholder="Name" />
  <PlanLimits bind:select={limits} />
  {#snippet footer()}
    <Buttons bind:open label="Safe" />
  {/snippet}
</Modal>

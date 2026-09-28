<script module lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import type { Type } from "../lib/types";

  const modal = $state<{
    open: boolean;
    id: string | undefined;
    part: number | null;
    name: string;
    limits: any;
    what: number | null;
    hook: number | null;
  }>({
    open: false,
    id: undefined,
    part: null,
    name: "",
    limits: {},
    what: null,
    hook: null,
  });

  const sethook = (type: Type, h: number | undefined) => {
    modal.what = type.id;
    modal.hook = h as number | null;
  };

  export function start(p: ServicePlan) {
    modal.id = p.id;
    modal.part = p.part;
    modal.name = p.name;
    modal.what = p.what;
    modal.hook = p.hook;
    modal.limits = p.to_object();
    modal.open = true;
  }
</script>

<script lang="ts">
  import { ButtonGroup, Input, InputAddon } from "flowbite-svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import GearForm from "../Widgets/GearForm.svelte";
  import type { Snippet } from "svelte";
  import PlanLimits from "./PlanLimits.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  interface Props {
    safePlan: (p: ServicePlan) => void;
    no_gear: boolean;
    children?: Snippet;
  }

  let { safePlan, no_gear, children }: Props = $props();

  function onaction() {
    let newplan = new ServicePlan({
      ...modal.limits,
      id: modal.id,
      part: modal.part,
      what: modal.what,
      name: modal.name,
      hook: modal.hook,
    });
    safePlan(newplan);
    modal.open = false;
  }
</script>

<Modal size="xs" bind:open={modal.open} {onaction}>
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
      <InputAddon>{m.attachform_of()}</InputAddon>
      <GearForm bind:gear={modal.part} />
    </ButtonGroup>
  {/if}
  <Input
    type="text"
    bind:value={modal.name}
    autofocus
    required
    placeholder={m.partform_name()}
  />
  <PlanLimits bind:select={modal.limits} />
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.gearcard_save()} />
  {/snippet}
</Modal>

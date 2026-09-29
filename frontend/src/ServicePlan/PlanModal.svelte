<script module lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { parts, type Part } from "../lib/part";
  import { getCategory, types, type Type } from "../lib/types";
  import { m } from "../../paraglide/messages";

  const modal = $state<{
    open: boolean;
    id: string | undefined;
    part: number | null;
    name: string;
    limits: any;
    what: number | null;
    hook: number | null;
    header: string;
    no_gear: boolean;
    safePlan: (p: ServicePlan) => void;
  }>({
    open: false,
    id: undefined,
    part: null,
    name: "",
    limits: {},
    what: null,
    hook: null,
    header: "",
    no_gear: false,
    safePlan: () => {},
  });

  const sethook = (type: Type, h: number | undefined) => {
    modal.what = type.id;
    modal.hook = h as number | null;
  };

  /**
   * The two ways into the plan modal. Each computes its mode — draft plan,
   * header, whether the gear section shows, and which operation saves —
   * then opens the one shared dialog, re-targeted for it.
   */
  export function newPlan(p: Part) {
    let no_gear = !p.isGear();
    let plan = no_gear
      ? new ServicePlan({ part: p.id, what: p.what, hook: null })
      : new ServicePlan({ part: p.id });
    open(plan, {
      header: m.newplan_header_part({ name: no_gear ? p.name : "" }),
      no_gear,
      safePlan: saveNew,
    });
  }

  export function updatePlan(p: ServicePlan) {
    let header: string;
    if (p.part) {
      let part = parts[p.part];
      if (part.isGear() && p.hook != null) {
        header = m.updateplan_header_hook_part({
          hook: types[p.what].human_name(p.hook),
          name: part.name,
        });
      } else {
        header = m.updateplan_header_part({ name: part.name });
      }
    } else {
      header = m.updateplan_header_generic({
        hook: types[p.what].human_name(p.hook),
        any: getCategory()!.localizedAnyDative(),
      });
    }
    open(new ServicePlan(p), { header, no_gear: true, safePlan: saveUpdate });
  }

  /** Open the shared dialog for the given plan and mode. */
  function open(
    p: ServicePlan,
    config: {
      header: string;
      no_gear: boolean;
      safePlan: (p: ServicePlan) => void;
    },
  ) {
    modal.id = p.id;
    modal.part = p.part;
    modal.name = p.name;
    modal.what = p.what;
    modal.hook = p.hook;
    modal.limits = p.to_object();
    modal.header = config.header;
    modal.no_gear = config.no_gear;
    modal.safePlan = config.safePlan;
    modal.open = true;
  }

  async function saveNew(p: ServicePlan) {
    await p.create();
  }

  async function saveUpdate(p: ServicePlan) {
    await p.update();
  }
</script>

<script lang="ts">
  import { ButtonGroup, Input, InputAddon } from "flowbite-svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import GearForm from "../Widgets/GearForm.svelte";
  import PlanLimits from "./PlanLimits.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

  function onaction() {
    let newplan = new ServicePlan({
      ...modal.limits,
      id: modal.id,
      part: modal.part,
      what: modal.what,
      name: modal.name,
      hook: modal.hook,
    });
    modal.safePlan(newplan);
    modal.open = false;
  }
</script>

<Modal size="xs" bind:open={modal.open} {onaction}>
  {#snippet header()}
    {modal.header}
  {/snippet}
  {#if !modal.no_gear}
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

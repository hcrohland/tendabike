<script module lang="ts">
  import { Type } from "../lib/types";
  import { getUser } from "../lib/user";
  import { Part } from "../lib/part";
  import { default_attach_date, prev_attach_time } from "../lib/attachment";

  const modal = $state<{
    open: boolean;
    part: any;
    gear: Part;
    type: Type | undefined;
    hook: number | undefined;
    single: boolean;
  }>({
    open: false,
    part: undefined,
    gear: new Part({}),
    type: undefined,
    hook: undefined,
    single: true,
  });

  // one date serves both roles for a part being created (purchase ≡ attach)
  function setDate() {
    modal.part.purchase = default_attach_date(
      modal.gear.id!,
      modal.hook,
      modal.type?.id,
    );
  }

  // walk the slot's attachment history; there is no part yet
  function prevdate(t: Date) {
    return prev_attach_time(
      t,
      undefined,
      modal.gear.id!,
      modal.hook,
      modal.type?.id,
    );
  }

  const setType = (t: Type, h: number | undefined) => {
    modal.part.what = t.id;
    modal.part.hook = h;
    modal.type = t;
    modal.hook = h;
    setDate();
  };

  export const installPart = (g: Part) => {
    modal.gear = g;
    modal.part = {
      ...new Part({
        owner: getUser()?.id,
      }),
    };
    modal.type = undefined;
    setDate();
    modal.open = true;
  };
</script>

<script lang="ts">
  import { InputAddon, ButtonGroup } from "flowbite-svelte";
  import NewForm from "../Part/PartForm.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function attachPart(part: Part | void) {
    if (!part) return;
    await part.attach(
      part.purchase,
      !modal.single,
      modal.gear!.id!,
      modal.hook!,
    );
  }

  async function onaction() {
    await new Part(modal.part).create().then(attachPart);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    <ButtonGroup class="col-md-12">
      <InputAddon>{m.action_new()}</InputAddon>
      <TypeForm
        onChange={setType}
        classes={{ select: "rounded-none h-full" }}
      />
      <InputAddon>{m.installpart_of({ gear: modal.gear.name })}</InputAddon>
    </ButtonGroup>
  {/snippet}

  <NewForm
    type={modal.type}
    bind:part={modal.part}
    mindate={modal.gear.purchase}
    {prevdate}
  />
  {#if modal.type?.is_hook()}
    <Switch bind:checked={modal.single}>{m.installpart_keep_attached()}</Switch>
  {/if}

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_install()} />
  {/snippet}
</Modal>

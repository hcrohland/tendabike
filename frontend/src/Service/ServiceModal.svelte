<script module lang="ts">
  import { Service } from "../lib/service";

  const modal = $state<{
    open: boolean;
    service: Service;
  }>({
    open: false,
    service: new Service({}),
  });

  export function start(s: Service) {
    modal.service = s;
    modal.open = true;
  }
</script>

<script lang="ts">
  import {
    Input,
    ButtonGroup,
    InputAddon,
    Checkbox,
    Listgroup,
    Textarea,
  } from "flowbite-svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { plansForPart, planCmp } from "../lib/serviceplan";
  import type { Snippet } from "svelte";
  import { parts } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  interface Props {
    saveService: (p: Service) => void;
    noname?: boolean;
    mindate?: Date;
    children?: Snippet;
  }

  let { saveService, noname = false, mindate, children }: Props = $props();

  let part = $derived(parts[modal.service.part_id]);

  let { name, notes, plans, time } = $derived(modal.service);

  let choices: any = $derived(
    plansForPart(modal.service.part_id, time)
      .sort(planCmp)
      .map((p) => ({
        value: p.id!,
        label: p.name,
        checked: modal.service.plans.some((q) => q == p.id),
      })),
  );

  function onaction() {
    Object.assign(modal.service, { name, notes, plans, time });
    saveService(modal.service);
  }
</script>

<Modal size="sm" bind:open={modal.open} {onaction}>
  {#snippet header()}
    {@render children?.()}
    {m.servicemodal_header({
      name: part.name,
      vendor: part.vendor,
      model: part.model,
    })}
  {/snippet}
  <!-- svelte-ignore a11y_autofocus -->
  <Input
    type="text"
    bind:value={name}
    disabled={noname}
    autofocus
    required
    placeholder={m.partform_name()}
  />
  <ButtonGroup>
    <Textarea bind:value={notes} placeholder={m.gearcard_notes()} />
  </ButtonGroup>
  <div class="flex">
    {#if choices.length > 0}
      <InputAddon>{m.servicemodal_resolves()}</InputAddon>
      <Listgroup class="gap-1 mx-2">
        <Checkbox bind:group={plans} {choices} />
      </Listgroup>
    {/if}
  </div>
  <ButtonGroup>
    <InputAddon class="text-end">{m.attachform_at()}</InputAddon>
    <DateTime bind:date={time} {mindate} required />
  </ButtonGroup>

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.gearcard_save()} />
  {/snippet}
</Modal>

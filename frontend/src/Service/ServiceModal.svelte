<script lang="ts">
  import {
    Input,
    ButtonGroup,
    Modal,
    Button,
    InputAddon,
    Checkbox,
    Listgroup,
    Textarea,
  } from "flowbite-svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { Service } from "../lib/service";
  import { plans as planstore, plans_for_part } from "../lib/serviceplan";
  import { attachments } from "../lib/attachment";
  import type { Snippet } from "svelte";
  import { parts } from "../lib/part";

  interface Props {
    saveService: (p: Service) => void;
    noname?: boolean;
    mindate?: Date;
    children?: Snippet;
  }

  let { saveService, noname = false, mindate, children }: Props = $props();

  let open = $state(false);
  let service = $state(new Service({}));
  let part = $derived($parts[service.part_id]);
  let choices: any = $state([]);

  let { name, notes, plans, time } = $derived(service);

  function onaction() {
    Object.assign(service, { name, notes, plans, time });
    saveService(service);
  }

  export function start(s: Service) {
    choices = plans_for_part($planstore, $attachments, s.part_id, s.time).map(
      (p) => ({
        value: p.id!,
        label: p.name,
        checked: s.plans.some((q) => q == p.id),
      }),
    );
    service = s;
    open = true;
  }
</script>

<Modal size="sm" bind:open form {onaction}>
  {#snippet header()}
    {@render children?.()} Service for {part.name}
    {part.vendor}
    {part.model}
  {/snippet}
  <!-- svelte-ignore a11y_autofocus -->
  <Input
    type="text"
    bind:value={name}
    disabled={noname}
    autofocus
    required
    placeholder="Name"
  />
  <ButtonGroup>
    <Textarea bind:value={notes} placeholder="Notes" />
  </ButtonGroup>
  <div class="flex">
    {#if choices.length > 0}
      <InputAddon>Resolves:</InputAddon>
      <Listgroup class="gap-1 mx-2">
        <Checkbox bind:group={plans} {choices} />
      </Listgroup>
    {/if}
  </div>
  <ButtonGroup>
    <InputAddon class="text-end">at</InputAddon>
    <DateTime bind:date={time} {mindate} required />
  </ButtonGroup>

  {#snippet footer()}
    <Button onclick={() => (open = false)} color="alternative">Cancel</Button>
    <Button type="submit" value="create" class="float-end">Safe</Button>
  {/snippet}
</Modal>

<script lang="ts">
  import { Input, Label, Textarea } from "flowbite-svelte";
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import DateTime from "../Widgets/DateTime.svelte";
  import { m } from "../../paraglide/messages";

  interface Props {
    type: Type | undefined;
    part: Part;
    maxdate?: Date | undefined;
    mindate?: Date | undefined;
  }

  let {
    type,
    part = $bindable(),
    maxdate = undefined,
    mindate = undefined,
  }: Props = $props();
</script>

<div>
  <Label class="mb-2">{m.partform_call_it()}</Label>
  <!-- svelte-ignore a11y_autofocus -->
  <Input
    type="text"
    class="form-control"
    bind:value={part.name}
    autofocus
    required
    placeholder={m.partform_name()}
  />
</div>
<div class="grid gap-4 md:grid-cols-2">
  <div>
    <Label class="mb-2">{m.partform_it_is_a()}</Label>
    <Input
      type="text"
      class="form-control"
      bind:value={part.vendor}
      placeholder={m.partform_brand()}
      required
    />
  </div>
  <div>
    <Label class="mb-2 invisible">...</Label>
    <Input
      type="text"
      class="form-control"
      bind:value={part.model}
      placeholder={m.partform_model()}
      required
    />
  </div>
  <div>
    <Label class="mb-2">
      {m.partform_new_day({ type: type?.localizedName() ?? "" })}
    </Label>
    <DateTime bind:date={part.purchase} {maxdate} {mindate} required rounded />
  </div>
  <div class="md:col-span-2">
    <Label class="mb-2">{m.gearcard_notes()}</Label>
    <Textarea
      class="w-full"
      bind:value={part.notes}
      placeholder={m.gearcard_notes_placeholder()}
      rows={3}
    />
  </div>
</div>

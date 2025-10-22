<script lang="ts">
  import { Input, Label } from "flowbite-svelte";
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import DateTime from "../Widgets/DateTime.svelte";

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
  <Label class="mb-2">You call it</Label>
  <!-- svelte-ignore a11y_autofocus -->
  <Input
    type="text"
    class="form-control"
    bind:value={part.name}
    autofocus
    required
    placeholder="Name"
  />
</div>
<div class="grid gap-4 md:grid-cols-2">
  <div>
    <Label class="mb-2">and it is a</Label>
    <Input
      type="text"
      class="form-control"
      bind:value={part.vendor}
      placeholder="Brand"
      required
    />
  </div>
  <div>
    <Label class="mb-2 invisible">...</Label>
    <Input
      type="text"
      class="form-control"
      bind:value={part.model}
      placeholder="Model"
      required
    />
  </div>
  <div>
    <Label class="mb-2">
      New {type?.name || ""} day was
    </Label>
    <DateTime bind:date={part.purchase} {maxdate} {mindate} required rounded />
  </div>
</div>

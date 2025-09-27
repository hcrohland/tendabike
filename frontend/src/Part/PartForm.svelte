<script lang="ts">
  import { Input, Label } from "flowbite-svelte";
  // import DateTime from "../Widgets/DateTime.svelte";
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
  <Label class="mb-2" for="inputName">You call it</Label>
  <!-- svelte-ignore a11y_autofocus -->
  <Input
    type="text"
    class="form-control"
    id="inputName"
    bind:value={part.name}
    autofocus
    required
    placeholder="Name"
  />
</div>
<div class="grid gap-4 md:grid-cols-2">
  <div>
    <Label class="mb-2" for="inputBrand">and it is a</Label>
    <Input
      type="text"
      class="form-control"
      id="inputBrand"
      bind:value={part.vendor}
      placeholder="Brand"
      required
    />
  </div>
  <div>
    <Label class="mb-2 invisible" for="inputModel">...</Label>
    <Input
      type="text"
      class="form-control"
      id="inputModel"
      bind:value={part.model}
      placeholder="Model"
      required
    />
  </div>
  <div>
    <Label class="mb-2" for="inputDate">
      New {type?.name || ""} day was
    </Label>
    <DateTime
      id="inputDate"
      bind:date={part.purchase}
      {maxdate}
      {mindate}
      required
    />
  </div>
</div>

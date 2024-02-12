<script lang="ts">
  import { Input, FormGroup } from "@sveltestrap/sveltestrap";
  import { Limits } from "./serviceplan";
  import { createEventDispatcher } from "svelte";
  import PlanSelect from "./PlanSelect.svelte";
  const dispatch = createEventDispatcher();

  export let limits = new Limits({});
  export let name: string = "";

  $: if (name.length > 0 && limits) {
    dispatch("change", { name, limits });
  }
</script>

<FormGroup class="col-md-12">
  <!-- svelte-ignore a11y-autofocus -->
  <Input
    type="text"
    class="form-control"
    id="inputName"
    bind:value={name}
    autofocus
    required
    placeholder="Name"
  />
</FormGroup>
<PlanSelect bind:select={limits} />

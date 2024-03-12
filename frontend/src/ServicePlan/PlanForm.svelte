<script lang="ts">
  import { FormGroup, Input } from "@sveltestrap/sveltestrap";
  import PlanLimits from "./PlanLimits.svelte";
  import { ServicePlan } from "../lib/serviceplan";
  export let plan: ServicePlan;
</script>

<FormGroup class="col-md-12">
  <!-- svelte-ignore a11y-autofocus -->
  <Input
    type="text"
    class="form-control"
    id="inputName"
    bind:value={plan.name}
    autofocus
    required
    placeholder="Name"
  />
</FormGroup>
<PlanLimits bind:select={plan} />

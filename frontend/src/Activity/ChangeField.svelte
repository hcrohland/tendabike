<script lang="ts">
  import { FormGroup, Input } from "@sveltestrap/sveltestrap";

  export let field: number | undefined;
  export let label: string;

  let required = field != undefined;
</script>

<FormGroup floating {label}>
  <Input bind:value={field} type="number" min="0" {required} />
</FormGroup>

<script lang="ts">
  import { InputGroupText, Input } from "@sveltestrap/sveltestrap";
  export let checked: boolean;
  export let id = "custominputneedsone";
</script>

<InputGroupText>
  <Input type="switch" {id} name="customSwitch" bind:checked></Input>
  <slot />
</InputGroupText>

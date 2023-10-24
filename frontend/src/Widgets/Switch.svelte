<InputGroupText>
  <Input
    type="switch"
    {id}
    name="customSwitch"
    bind:checked>
  </Input>
  <slot />
</InputGroupText>

<script lang="ts">
import {InputGroupText, Input} from 'sveltestrap'
 export let checked
 export let id = "custominputneedsone"
</script>
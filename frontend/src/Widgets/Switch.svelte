  <InputGroupAddon addonType="prepend">
    <InputGroupText>
      <CustomInput
      type="switch"
      {id}
      name="customSwitch"
      bind:checked>
       <slot />
      </CustomInput>
    </InputGroupText>
  </InputGroupAddon>

<script lang="ts">
import {InputGroupText, InputGroupAddon, CustomInput} from 'sveltestrap'
 export let checked
 export let id = "custominputneedsone"
</script>
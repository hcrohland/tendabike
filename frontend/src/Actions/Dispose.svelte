  <InputGroupAddon addonType="prepend">
    <InputGroupText>
      <CustomInput
      type="switch"
      id="custominputneedsone"
      name="customSwitch"
      bind:checked={dispose}>
      {#if dispose}
        disposing <slot />
      {:else}
        dispose <slot />?
      {/if}
      </CustomInput>
    </InputGroupText>
  </InputGroupAddon>

<script lang="ts">
  import {InputGroupText, InputGroupAddon, CustomInput} from 'sveltestrap'
 export let dispose
</script>
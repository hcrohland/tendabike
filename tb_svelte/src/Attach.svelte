<script>
  import Modal from './Modal.svelte';
  import DateTime from './DateTime.svelte';
  import {mypatch, filterValues, types, parts} from './store.js';
  
  export let part;
  import { createEventDispatcher } from 'svelte';

	const dispatch = createEventDispatcher();


  let type = $types[part.what];

  let attach;
  let showModal;
  close();

  if (type.hooks.length == 1) attach.hook = type.hooks[0];

  let disabled = true; 


  async function attachPart () {
    disabled = true;
    await mypatch('/attach/', attach)
      .then(data => parts.updateMap(data))
      .then(dispatch('refresh'))
      .then(close)
  }

  function close () {
    showModal = false;
    attach = {
      part_id: part.id,
      attached: part.purchase,
      gear: undefined,
      hook: undefined,
    } 
  }

  // $: if (showModal) console.log(attach)
  $: disabled = !($types[attach.hook] && $parts[attach.gear])
</script>

<span type="button" class="badge badge-secondary float-right" on:click={() => showModal = true}>
  attach
</span>

{#if showModal}
  <Modal save="Attach" on:close={close}>
    <span slot="header"> Attach {type.name} {part.name} {part.vendor} {part.model} </span>
    <form>
      <div class="form-inline">
        <div class="form-inline">
          <div class="input-group mb-0 mr-sm-2 mb-sm-2">
            <div class="input-group-prepend">
              <span class="input-group-text">to</span>
            </div>
            {#if type.hooks.length > 1}
              <select name="hook" class="form-control" required bind:value={attach.hook}>
                <option hidden value={undefined}> -- select one -- </option>
                {#each type.hooks as h}
                  <option value={h}>{$types[h].name}</option>
                {/each}
              </select>
                    <!-- </div> -->
                    <!-- <div class="input-group mb-2 mr-sm-2"> -->
              <div class="input-group-prepend">
                <span class="input-group-text">of</span>
              </div>
            {/if}
            <select name="gear" class="form-control" required bind:value={attach.gear}>
              <option hidden value> -- select one -- </option>
              {#each filterValues($parts, (p) => type.main == p.what) as gear}
                <option value={gear.id}>{gear.name}</option>
              {/each}
            </select> 
          </div>  
        </div>  
        <div class="form-inline ">
          <!-- <input type="hidden" name="tz" value={{ now() | date(format="%:z")}}> -->
          <br>
          <DateTime bind:date={attach.attached}/> 
        </div>
      </div>
    </form> 
    <span slot="footer">
      <button type="submit" class="btn btn-primary float-right" {disabled} on:click={attachPart}>Attach</button>
    </span>
  </Modal>
{/if}
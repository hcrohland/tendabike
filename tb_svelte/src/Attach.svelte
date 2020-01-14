<script>
  import Modal from './Modal.svelte';
  import DateTime from './DateTime.svelte';
  import {myfetch, filterValues, types, parts} from './store.js';
  
  export let part;

  let name;
  let vendor;
  let model;
  let purchase;
  let attached = part.purchase;

  let showModal = false;

  let type = $types[part.what];

  function attachPart () {
    alert("Not attached at " + attached);
    showModal = false;
  }

</script>

<span type="button" class="badge badge-secondary float-right" on:click="{() => showModal = true}">
  attach
</span>

{#if showModal}
  <Modal save="Attach" on:close="{() => showModal = false}">
    <span slot="header"> Attach {type.name} {part.name} {part.vendor} {part.model} </span>
    <form>
      <div class="form-inline">
        <div class="form-inline">
          <div class="input-group mb-0 mr-sm-2 mb-sm-2">
            <div class="input-group-prepend">
              <span class="input-group-text">to</span>
            </div>
            {#if type.hooks.length > 1}
              <select name="hook" class="form-control" required>
                <option hidden value> -- select one -- </option>
                {#each type.hooks as h}
                  <option value={{h}}>{$types[h].name}</option>
                {/each}
              </select>
                    <!-- </div> -->
                    <!-- <div class="input-group mb-2 mr-sm-2"> -->
              <div class="input-group-prepend">
                <span class="input-group-text">of</span>
              </div>
            {:else}
              <input type="hidden" name="hook" value={type.hooks[0]}>
            {/if}
            <select name="gear" class="form-control" required>
              <option hidden value> -- select one -- </option>
              {#each filterValues($parts, (p) => type.main == p.what) as gear}
                <option value={gear.id}>{gear.name}</option>
              {/each}
            </select> 
          </div>  
        </div>  
        <div class="form-inline ">
          <!-- <input type="hidden" name="tz" value={{ now() | date(format="%:z")}}> -->
          <input type="hidden" name="part_id" value={ part.id }>
          <br>
          <DateTime bind:date={attached}/> 
        </div>
      </div>
    </form> 
    <span slot="footer">
      <button type="submit" class="btn btn-primary float-right" on:click={attachPart}>Attach</button>
    </span>
  </Modal>
{/if}
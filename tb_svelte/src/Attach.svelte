<script>
  import Modal from './Modal.svelte';
  import DateTime from './DateTime.svelte';
  import {myfetch, updatePartAttach, attachments, initData, filterValues, types, parts, by} from './store';
  
  import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

  export let part;
  export let title = "attach";

  let attach, showModal, disabled;
  
  const type = $types[part.what];


  function lastDetach(part) {
    let last = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))[0]
    
    if (last) {
      return last.detached ? last.detached : last.attached
    }else {
      return part.purchase
    }
  }

  async function attachPart () {
    disabled = true;
    try {
      await myfetch('/attach/', 'PATCH', attach)
        .then(data => updatePartAttach(data))
      dispatch('saved')
    } catch (e) {
      alert (e)
      initData()
    }
    showModal = false;
  }

  function popup () {
    showModal = true;
    disabled = true;
    attach = {
      part_id: part.id,
      attached: lastDetach(part),
      gear: undefined,
      hook: (type.hooks.length == 1) ? type.hooks[0] : undefined,
    } 
  }

  // $: if (showModal) console.log(attach)
  $: disabled = attach && !($types[attach.hook] && $parts[attach.gear])
</script>

<span type="button" class="badge badge-secondary float-right" on:click={popup}>
  {title}
</span>

{#if showModal}
  <Modal on:close={() => showModal = false}>
    <span slot="header"> Attach {type.name} {part.name} {part.vendor} {part.model} </span>
    <form>
        <div class="form-inline">
          <div class="input-group mb-0 mr-sm-2 mb-sm-2">
            <div class="input-group-prepend">
              <span class="input-group-text">to</span>
            </div>
            {#if type.hooks.length > 1}
              <!-- svelte-ignore a11y-autofocus -->
              <select name="hook" class="form-control" autofocus required bind:value={attach.hook}>
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
        <!-- </div>  
        <div class="form-inline "> -->
          <!-- <input type="hidden" name="tz" value={{ now() | date(format="%:z")}}> -->
          <div class="input-group mb-0 mr-sm-2 mb-sm-2">
          <div class="input-group-prepend">
              <span class="input-group-text">at</span>
          </div>
          <DateTime class="input-group-text" bind:date={attach.attached}/> 
        </div>
        </div>
    </form> 
    <span slot="footer">
      <button type="submit" class="btn btn-primary float-right" {disabled} on:click={attachPart}>Attach</button>
    </span>
  </Modal>
{/if}
<script lang="ts">
import {
    Input, InputGroup, CustomInput, Table, Container, Button
  } from 'sveltestrap';
import type {AttEvent, Attachment, Part, Type} from '../types'
import {category, filterValues, types, handleError, updateSummary, myfetch} from '../store'

export let gear: Part;
export let attachees: Attachment[];

type Group = {
  group: string;
  enabled: boolean;
  types: Type[];
  vendor: string;
  model: string;
}

const groupBy = function(xs: Type[]) {
  return xs.reduce(function(rv : Group[], x) {
    (rv[x.group] = rv[x.group] || {types: [], group:x.group, vendor: undefined, model: undefined, enabled: false}).types.push(x);
    return rv;
  }, []);
};

function groupAvailable(group: Group) {
  let res = true
  group.types.forEach(t => {
    if (attachees.find((a) => {return a.what == t.id})) {
        res = false
      }
  });
  return res
}

let allgroups = Object.values(groupBy(filterValues(types, (t) => t.group && t.main == gear.what)))
let groups = allgroups.filter(groupAvailable)

// Vendor needs to be set for any enabled group
$: disabled = !groups.reduce((r: boolean, v: Group) => {
  return r && (!v.enabled || (v.enabled && v.vendor != ''))
}, true)

async function attachPart (part, hook) {
    let attach : AttEvent = {
     part_id: part.id,
     time: part.purchase,
     gear: gear.id,
     hook: hook,
    }
    
    await myfetch('/part/attach', 'POST', attach)
        .then(updateSummary)
  }

  async function installPart (newpart:Part, hook: number) {
    disabled = true;
    await myfetch('/part/', 'POST', newpart)
      .then((p) => attachPart(p, hook))
      .catch(handleError)
  }

function setGroup (g: Group) {
  if (!g.enabled) return;

  let p: Part = Object.assign({}, gear);
  
  p.id = undefined;
  p.name = g.vendor + ' ' + g.model
  p.vendor =  g.vendor;
  p.model = g.model;
  g.types.forEach(t => {
    p.what = t.id      
    t.hooks.forEach(h => {
      installPart(p, h)
    });
  });
}

function save () {
  groups.forEach(g => {
    if (g.enabled) setGroup(g)
  });
  show_button = true
}
let show_button = (groups.length != allgroups.length)

</script>

{#if gear.disposed_at == null && groups.length > 0}
<Container>
  {#if show_button}
    <Button on:click={() => show_button = false}>
      Add more component groups
    </Button>
  {:else}
    <Table borderless>
      <tr>
        <th colspan=80>
          Add components groups:
        </th>
      </tr>
      {#each groups as g, i}
      <tr>
        <th style="vertical-align: middle">
          <CustomInput type="switch" id={g.group} name="customSwitch" bind:checked={g.enabled}> 
            {g.group}:
          </CustomInput>
        </th>
        <td>
          <InputGroup>
            <Input type="text" class="form-control" id="inputBrand" bind:value={g.vendor} placeholder="Brand" disabled={!g.enabled}/>
            <Input type="text" class="form-control" id="inputModel" bind:value={g.model} placeholder="Model" disabled={!g.enabled}/>
          </InputGroup>
        </td>
      </tr>
      {/each}
    </Table>
    <Button {disabled} on:click={save}> Set </Button>
  {/if}
  </Container>
{/if}

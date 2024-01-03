<script lang="ts">
import { Button, Col, InputGroup, Row } from "@sveltestrap/sveltestrap";

import { activities, filterValues, handleError, myfetch, parts, updateSummary } from "../store";
import type { Type } from "../types";
import ActivityList from "../Widgets/ActivityList.svelte";

export let type: Type

let isOpen = false
let value;

function defaultGear(id: number) {
    myfetch('/activ/defaultgear', 'POST', id)
      .then(updateSummary)
      .catch(handleError)
  }

$: unassigned = filterValues($activities, (a) => !a.gear && type.acts.some((t) => t.id == a.what))
</script>
{#if unassigned.length > 0}
<Col md=8 lg=6>
  <Row>
    <InputGroup>
      <button on:click={() => isOpen = true}>
        Assign {unassigned.length} unassigned activities to
      </button>
      <select name="gear" class="form-control" required bind:value>
        <option hidden value> -- select one -- </option>
        {#each filterValues($parts, (p) => p.what == type.id) as gear}
        <option value={gear.id}>{gear.name}</option>
        {/each}
      </select> 
      <Button disabled={!value} on:click={() => defaultGear(value)}> Ok </Button>
    </InputGroup>
  </Row>
</Col>
<ActivityList activities={unassigned} bind:isOpen>Unassigned Activities</ActivityList>
{/if}
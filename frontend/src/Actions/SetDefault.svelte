<script lang="ts">
import { Button, Col, InputGroup, InputGroupAddon, InputGroupText, Row } from "sveltestrap";

import { activities, filterValues, handleError, myfetch, parts, updateSummary } from "../store";

import type { Type } from "../types";

export let type: Type

let value;

function defaultGear(id: number) {
    myfetch('/activ/defaultgear', 'POST', id)
      .then(updateSummary)
      .catch(handleError)
  }

$: unassigned = filterValues($activities, (a) => !a.gear)
</script>
{#if unassigned.length > 0}
<Col md=8 lg=6>
  <Row>
    <InputGroup>
      <InputGroupAddon addonType="prepend">
        <InputGroupText>Assign {unassigned.length} unassigned activities to</InputGroupText>
      </InputGroupAddon>
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
{/if}
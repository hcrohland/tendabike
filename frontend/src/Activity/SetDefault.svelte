<script lang="ts">
  import { Button, Col, InputGroup, Row } from "@sveltestrap/sveltestrap";

  import { handleError, myfetch, updateSummary } from "../lib/store";
  import { Type } from "../lib/types";
  import ActivityList from "./ActivityList.svelte";
  import { filterValues } from "../lib/mapable";
  import { activities } from "./activity";
  import { parts } from "../Part/part";

  export let type: Type;

  let isOpen = false;
  let value: number;

  function defaultGear(id: number) {
    myfetch("/activ/defaultgear", "POST", id)
      .then(updateSummary)
      .catch(handleError);
  }

  $: unassigned = filterValues(
    $activities,
    (a) => !a.gear && type.acts.some((t) => t.id == a.what),
  );
</script>

{#if unassigned.length > 0}
  <Col md="8" lg="6">
    <Row>
      <InputGroup>
        <button on:click={() => (isOpen = true)}>
          Assign {unassigned.length} unassigned activities to
        </button>
        <select name="gear" class="form-control" required bind:value>
          <option hidden value> -- select one -- </option>
          {#each type.parts($parts) as gear}
            <option value={gear.id}>{gear.name}</option>
          {/each}
        </select>
        <Button disabled={!value} on:click={() => defaultGear(value)}>
          Ok
        </Button>
      </InputGroup>
    </Row>
  </Col>
  <ActivityList activities={unassigned} bind:isOpen>
    Unassigned Activities
  </ActivityList>
{/if}

<script lang="ts">
  // import { slide } from 'svelte/transition';
  import {
    Card,
    CardBody,
    CardHeader,
    Col,
    Row,
  } from "@sveltestrap/sveltestrap";
  import { link } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import { fmtDate, fmtNumber, fmtSeconds } from "../lib/store";
  import { types } from "../lib/types";
  import { Usage, usages } from "../lib/usage";

  export let part: Part;
  export let display = false;

  let isOpen = false;
  let showLink = false;

  $: usage = $usages[part.usage] ? $usages[part.usage] : new Usage();

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return "unknown model";
    } else {
      return part.vendor + " " + part.model;
    }
  }
</script>

<Card>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="header"
    on:click={() => (isOpen = !isOpen)}
    on:mouseenter={() => (showLink = true)}
    on:mouseleave={() => (showLink = false)}
  >
    <CardHeader class="h5 mb-0">
      <Row>
        <Col>
          {part.name}
        </Col>
        {#if showLink || display}
          <Col>
            <slot />
          </Col>
        {/if}
      </Row>
    </CardHeader>
  </div>
  {#if isOpen || display}
    <!-- conflict with spa-router -->
    <!-- <div transition:slide> -->
    <CardBody>
      is a <span class="param">{model(part)}</span>
      {#if part.what == 1}
        <a href={"/strava/bikes/" + part.id} target="_blank"
          ><img
            src="strava_grey.png"
            alt="View on Strava"
            title="View on Strava"
          />
        </a>
      {/if}
      {#if part.what != types[part.what].main}
        {types[part.what].name.toLowerCase()}
      {/if}
      {#if !part.disposed_at}
        purchased <span class="param">{fmtDate(part.purchase)}</span>
        which
      {:else}
        you owned from <span class="param">{fmtDate(part.purchase)}</span>
        until <span class="param">{fmtDate(part.disposed_at)}</span>
        and
      {/if}
      you used
      <a href={"/activities/" + part.id} use:link class="param text-reset"
        >{fmtNumber(usage.count)}</a
      >
      times for <span class="param">{fmtSeconds(usage.time)}</span> hours.
      <p>
        You covered <span class="param"
          >{fmtNumber(parseFloat((usage.distance / 1000).toFixed(1)))}</span
        >
        km climbing <span class="param">{fmtNumber(usage.climb)}</span> and
        descending <span class="param">{fmtNumber(usage.descend)}</span> meters
        {#if usage.energy > 0}
          and expended
          <span class="param">{fmtNumber(usage.energy)}</span> kiloJoules of energy
        {/if}
      </p>
    </CardBody>
    <!-- </div> -->
  {/if}
</Card>

<style>
  .header:hover {
    background-color: lightgray;
  }

  .param {
    font-weight: bold;
  }
</style>

<script lang="ts">
  // import { slide } from 'svelte/transition';
  import {
    Card,
    CardBody,
    CardHeader,
    Row,
    Col,
  } from "@sveltestrap/sveltestrap";
  import { fmtSeconds, fmtDate, fmtNumber } from "../lib/store";
  import { Part } from "../lib/part";
  import { link } from "svelte-spa-router";
  import { usages, Usage } from "../lib/usage";
  import { types } from "../lib/types";

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
      </p></CardBody
    >
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

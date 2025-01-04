<script lang="ts">
  export let date = new Date();
  export let mindate: any = undefined;
  export let maxdate: any = undefined;
  export let id: any = undefined;
  export let required: any = undefined;
  export let prevdate: ((t: Date) => Date) | undefined = undefined; // only usable w/o mindate

  // @ts-ignore
  import { DateInput } from "date-picker-svelte";
  import { roundTime } from "../lib/store";

  const options = {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
  };

  let now = roundTime(new Date());

  function handleChange(event: any) {
    const [selectedDates] = event.detail;
    date = selectedDates[0] as Date;
  }

  date = roundTime(date);
</script>

<DateInput
  value={date}
  min={mindate}
  max={maxdate}
  format="dd.MM.yy HH:mm"
  on:change={handleChange}
  {id}
  {required}
/>

{#if mindate}
  <button on:click|preventDefault={() => (date = mindate)}> &#706; </button>
{:else if prevdate}
  <button on:click|preventDefault={() => (date = prevdate(date))}>
    &#706;
  </button>
{/if}
{#if !(mindate && mindate > now)}
  <button on:click|preventDefault={() => (date = roundTime(new Date()))}>
    &#8226;
  </button>
{/if}

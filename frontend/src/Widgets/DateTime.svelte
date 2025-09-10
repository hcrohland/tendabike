<script lang="ts">
  export let date = new Date();
  export let mindate: any = undefined;
  export let maxdate: any = undefined;
  export let prevdate: ((t: Date) => Date) | undefined = undefined; // only usable w/o mindate

  const props = Object.assign({}, $$props);
  delete props.date;

  import SveltyPicker from "svelty-picker";
  import { roundTime } from "../lib/store";

  const options = {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
  };

  mindate = mindate ? roundTime(mindate) : undefined;
  maxdate = maxdate ? roundTime(maxdate) : undefined;
  let now = roundTime(new Date());

  let flatpickrOptions = {
    time_24hr: true,
    enableTime: true,
    minuteIncrement: 15,
    dateFormat: "j. M Y H:i",
    minDate: mindate,
    maxDate: maxdate,
  };

  function handleChange(event: any) {
    const [selectedDates] = event.detail;
    date = selectedDates[0] as Date;
  }

  date = roundTime(date);
  const minuteIncrement = 15;
</script>

<SveltyPicker
  initialDate={date}
  on:change={(e) => (date = new Date(e.detail))}
  mode="datetime"
  format="d. M yy h:ii"
  {minuteIncrement}
/>
<!-- hack to prevent spurious button clicks -->
<button
  hidden
  on:click|preventDefault|stopPropagation={() => {}}
  tabindex="-1"
/>
{#if mindate}
  <button on:click|preventDefault={() => (date = mindate)}> &#706; </button>
{:else if prevdate}
  <button on:click|preventDefault={() => (date = prevdate(date))}>
    &#706;
  </button>
{/if}
{#if !(mindate && mindate > now) && !(maxdate && maxdate < now)}
  <button on:click|preventDefault={() => (date = now)}> &#8226; </button>
{/if}
{#if maxdate}
  <button on:click|preventDefault={() => (date = maxdate)}> &#707; </button>
{/if}

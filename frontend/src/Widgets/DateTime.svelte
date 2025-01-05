<script lang="ts">
  export let date = new Date();
  export let mindate: any = undefined;
  export let prevdate: ((t: Date) => Date) | undefined = undefined; // only usable w/o mindate

  const props = Object.assign({}, $$props);
  delete props.date;

  import Flatpickr from "svelte-flatpickr";

  import "flatpickr/dist/flatpickr.css";
  import "flatpickr/dist/themes/light.css";
  import { roundTime } from "../lib/store";

  const options = {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
  };

  mindate = mindate ? roundTime(mindate) : mindate;
  let now = roundTime(new Date());

  let flatpickrOptions = {
    time_24hr: true,
    enableTime: true,
    minuteIncrement: 15,
    dateFormat: "j. M Y H:i",
    minDate: mindate,
  };

  function handleChange(event: any) {
    const [selectedDates] = event.detail;
    date = selectedDates[0] as Date;
  }

  date = roundTime(date);
</script>

<Flatpickr
  options={flatpickrOptions}
  value={date}
  children={props.children}
  on:change={handleChange}
  {...props}
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
{#if !(mindate && mindate > now)}
  <button on:click|preventDefault={() => (date = roundTime(new Date()))}>
    &#8226;
  </button>
{/if}

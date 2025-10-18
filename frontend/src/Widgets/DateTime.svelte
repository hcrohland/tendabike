<script lang="ts">
  import { preventDefault } from "svelte/legacy";

  import SveltyPicker, { formatDate, parseDate } from "svelty-picker";
  import { en } from "svelty-picker/i18n";
  import { roundTime } from "../lib/store";
  import { Button, ButtonGroup } from "flowbite-svelte";

  type Props = {
    date?: any;
    mindate?: Date;
    maxdate?: Date;
    prevdate?: (t: Date) => Date; // only usable w/o mindate
    id?: string;
    required?: boolean;
  };

  let {
    date = $bindable(new Date()),
    mindate = undefined,
    maxdate = undefined,
    prevdate = undefined,
    id: inputId = undefined,
    required = undefined,
  }: Props = $props();

  mindate = mindate ? roundTime(mindate) : undefined;
  maxdate = maxdate ? roundTime(maxdate) : undefined;
  let now = roundTime(new Date());

  const options = {
    // time_24hr: true,
    minuteIncrement: 15,
    format: "d. M yyyy - h:ii",
    startDate: mindate,
    endDate: maxdate,
    displayFormat: "d. M yyyy - h:ii",
    displayFormatType: "standard",
    // manualInput: true,
  };

  date = roundTime(date);
</script>

<ButtonGroup>
  <SveltyPicker
    bind:value={
      () => {
        return formatDate(date, options.format, en, "standard");
      },
      (v) => {
        date = v ? parseDate(v, options.format, en, "standard") : null;
      }
    }
    placeholder={formatDate(date, options.format, en, "standard")}
    mode="datetime"
    {inputId}
    {required}
    {...options}
  />

  {#if mindate}
    <Button onclick={preventDefault(() => (date = mindate))}>&#706;</Button>
  {:else if prevdate}
    <Button onclick={preventDefault(() => (date = prevdate(date)))}>
      &#706;
    </Button>
  {/if}
  {#if !(mindate && mindate > now) && !(maxdate && maxdate < now)}
    <Button onclick={preventDefault(() => (date = now))}>&#8226;</Button>
  {/if}
  {#if maxdate}
    <Button onclick={preventDefault(() => (date = maxdate))}>&#707;</Button>
  {/if}
</ButtonGroup>

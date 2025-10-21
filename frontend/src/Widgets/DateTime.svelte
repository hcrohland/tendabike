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
    required?: boolean;
    rounded?: boolean;
  };

  let {
    date = $bindable(roundTime(new Date())),
    mindate = undefined,
    maxdate = undefined,
    prevdate = undefined,
    required = undefined,
    rounded = false,
  }: Props = $props();

  mindate = mindate ? roundTime(mindate) : undefined;
  maxdate = maxdate ? roundTime(maxdate) : undefined;
  let now = roundTime(new Date());

  let inputClasses = $derived(
    "dark:bg-gray-700 " + (rounded ? "rounded-l-md" : "rounded-none"),
  );

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
    {required}
    {inputClasses}
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

<style>
  :global(.dark) {
    --sdt-bg-main: var(--color-gray-700);
    --sdt-shadow-color: #777;
    --sdt-color: #eee;
    --sdt-clock-color: var(--sdt-color);
    --sdt-clock-color-hover: var(--sdt-color);
    --sdt-clock-time-bg: transparent;
    --sdt-clock-time-bg-hover: transparent;
    --sdt-clock-disabled: #b22222;
    --sdt-clock-disabled-bg: var(--sdt-bg-main);
    --sdt-clock-selected-bg: var(--sdt-bg-selected);
    --sdt-header-color: #eee;
    --sdt-bg-selected: var(--color-primary-700);
    --sdt-table-disabled-date: #b22222;
    --sdt-table-disabled-date-bg: var(--sdt-bg-main);
    --sdt-table-data-bg-hover: var(--color-primary-800);
    --sdt-table-selected-bg: var(--sdt-bg-selected);
    --sdt-header-btn-bg-hover: #777;
    --sdt-color-selected: #fff;
    --sdt-table-today-indicator: #ccc;
    --sdt-clock-bg: #999;
    /* custom buttons */
    --sdt-today-bg: #e4a124;
    --sdt-today-color: #fff;
    --sdt-clear-color: #666;
    --sdt-clear-bg: #ddd;
    --sdt-clear-hover-color: #fff;
    --sdt-clear-hover-bg: #dc3545;
  }
  :global(.light) {
    --sdt-bg-main: #fff;
    --sdt-shadow-color: #ccc;
    --sdt-color: inherit;
    --sdt-clock-color: var(--sdt-color);
    --sdt-clock-color-hover: var(--sdt-color);
    --sdt-clock-time-bg: transparent;
    --sdt-clock-time-bg-hover: transparent;
    --sdt-clock-disabled: #b22222;
    --sdt-clock-disabled-bg: var(--sdt-bg-main);
    --sdt-clock-selected-bg: var(--sdt-bg-selected);
    --sdt-bg-selected: #286090;
    --sdt-table-disabled-date: #b22222;
    --sdt-table-disabled-date-bg: var(--sdt-bg-main);
    --sdt-table-data-bg-hover: #eee;
    --sdt-table-selected-bg: var(--sdt-bg-selected);
    --sdt-header-btn-bg-hover: #dfdfdf;
    --sdt-color-selected: #fff;
    --sdt-table-today-indicator: #ccc;
    --sdt-clock-bg: #eeeded;
    /* custom buttons */
    --sdt-today-bg: #1e486d;
    --sdt-today-color: #fff;
    --sdt-clear-color: #dc3545;
    --sdt-clear-bg: #fff;
    --sdt-clear-hover-color: #fff;
    --sdt-clear-hover-bg: #dc3545;
  }
</style>

<script lang="ts">
  import { preventDefault } from "svelte/legacy";

  import { roundTime } from "../lib/utils";
  import { Button, ButtonGroup, Datepicker, Timepicker } from "flowbite-svelte";
  import {
    AngleLeftOutline,
    AngleRightOutline,
    ClockOutline,
  } from "flowbite-svelte-icons";
  import { getLocale } from "../../paraglide/runtime";

  type Props = {
    date?: any;
    mindate?: Date;
    maxdate?: Date;
    // the history-walk step; shown in place of the min-jump button,
    // disabled while it has no earlier boundary
    prevdate?: (t: Date) => Date | undefined;
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

  // the client locale drives the Datepicker's display (issue #483: the date
  // must show in the client's locale, so a German client sees "8.10.2026")
  const locale = getLocale();
  const min = () => (mindate ? roundTime(mindate) : undefined);
  const max = () => (maxdate ? roundTime(maxdate) : undefined);
  let now = roundTime(new Date());

  let inputClasses = $derived(
    "bg-surface-3 " + (rounded ? "rounded-l-md" : "rounded-none"),
  );

  // `date` is split across the two pickers: the Datepicker owns the day, the
  // Timepicker owns the HH:mm. Each binds through an accessor that merges its
  // half back into `date`, preserving the other half.
  const datePart = $derived(
    date
      ? new Date(date.getFullYear(), date.getMonth(), date.getDate())
      : undefined,
  );
  const timePart = $derived(
    date
      ? `${String(date.getHours()).padStart(2, "0")}:${String(
          date.getMinutes(),
        ).padStart(2, "0")}`
      : "00:00",
  );

  function setDatePart(day: Date | undefined) {
    if (!day) return;
    const h = date ? date.getHours() : 0;
    const m = date ? date.getMinutes() : 0;
    date = new Date(day.getFullYear(), day.getMonth(), day.getDate(), h, m);
  }
  function setTimePart(hhmm: string) {
    if (!hhmm) return;
    const [h, m] = hhmm.split(":").map(Number);
    const base = date ?? new Date();
    date = new Date(base.getFullYear(), base.getMonth(), base.getDate(), h, m);
  }
</script>

<ButtonGroup>
  <Datepicker
    bind:value={
      () => {
        return datePart;
      },
      (v) => {
        setDatePart(v);
      }
    }
    {locale}
    availableFrom={min()}
    availableTo={max()}
    inputClass={inputClasses}
    required={required ?? false}
  />
  <Timepicker
    bind:value={
      () => {
        return timePart;
      },
      (v) => {
        setTimePart(v);
      }
    }
    inputClass={inputClasses}
    required={required ?? false}
  />

  {#if prevdate}
    {@const next = prevdate(date)}
    <Button
      disabled={!next}
      onclick={preventDefault(() => (date = next ?? date))}
    >
      <AngleLeftOutline class="shrink-0 h-5 w-5" />
    </Button>
  {:else if mindate}
    <Button onclick={preventDefault(() => (date = min()))}>
      <AngleLeftOutline class="shrink-0 h-5 w-5" />
    </Button>
  {/if}
  {#if !(mindate && mindate > now) && !(max() && max()! < now)}
    <Button onclick={preventDefault(() => (date = now))}>
      <ClockOutline class="shrink-0 h-5 w-5" />
    </Button>
  {/if}
  {#if maxdate}
    <Button onclick={preventDefault(() => (date = max()))}>
      <AngleRightOutline class="shrink-0 h-5 w-5" />
    </Button>
  {/if}
</ButtonGroup>

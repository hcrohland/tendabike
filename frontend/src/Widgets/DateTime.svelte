<script module lang="ts">
  // each instance gets a unique trigger id so the Popover's triggeredBy
  // selector never matches a sibling widget's button
  let seq = 0;
</script>

<script lang="ts">
  import { preventDefault } from "svelte/legacy";

  import { roundTime } from "../lib/utils";
  import {
    Button,
    ButtonGroup,
    Datepicker,
    Input,
    Popover,
  } from "flowbite-svelte";
  import {
    AngleLeftOutline,
    AngleRightOutline,
    CalendarMonthOutline,
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
    required = false,
    rounded = false,
  }: Props = $props();

  const btnId = `datetime-${seq++}`;

  // the client locale drives the Datepicker's display, so the date shows in
  // the client's locale (a German client sees "8.10.2026")
  const locale = getLocale();
  const min = () => (mindate ? roundTime(mindate) : undefined);
  const max = () => (maxdate ? roundTime(maxdate) : undefined);
  let now = roundTime(new Date());
  let open = $state(false);

  // the bound date's time snapped to the 15-minute grid; roundTime mutates
  // in place, so it runs on a copy — never the bound prop
  const gridDate = $derived(date ? roundTime(new Date(date)) : undefined);
  const datePart = $derived(
    gridDate
      ? new Date(
          gridDate.getFullYear(),
          gridDate.getMonth(),
          gridDate.getDate(),
        )
      : undefined,
  );
  const timePart = $derived(
    gridDate
      ? `${String(gridDate.getHours()).padStart(2, "0")}:${String(
          gridDate.getMinutes(),
        ).padStart(2, "0")}`
      : "00:00",
  );
  const label = $derived(
    date
      ? `${datePart!.toLocaleDateString(locale)} ${timePart}`
      : "Pick date & time",
  );

  function setDatePart(day: Date | undefined) {
    if (!day) return;
    // carry the rounded time over so the merged date stays on the grid
    const h = gridDate ? gridDate.getHours() : 0;
    const m = gridDate ? gridDate.getMinutes() : 0;
    date = new Date(day.getFullYear(), day.getMonth(), day.getDate(), h, m);
  }
  function setTimePart(hhmm: string) {
    if (!hhmm) return;
    const [h, m] = hhmm.split(":").map(Number);
    const base = date ?? new Date();
    // snap the chosen time to the 15-minute grid, as the picker always did
    date = roundTime(
      new Date(base.getFullYear(), base.getMonth(), base.getDate(), h, m),
    );
  }
</script>

<ButtonGroup>
  <Button id={btnId} color="light" class={rounded ? "rounded-l-md" : ""}>
    <CalendarMonthOutline class="me-2 h-4 w-4" />
    {label}
  </Button>

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

<Popover
  triggeredBy={`#${btnId}`}
  trigger="click"
  placement="bottom-start"
  bind:isOpen={open}
  class="p-3"
>
  <Datepicker
    inline
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
  />
  <div class="mt-3">
    <Input
      type="time"
      {required}
      bind:value={
        () => {
          return timePart;
        },
        (v) => {
          setTimePart(v);
        }
      }
    />
  </div>
</Popover>

<script lang="ts">
  import { DAY, fmtNumber, fmtSeconds } from "../lib/store";
  import SvelteTable from "../Widgets/SvelteTable.svelte";
  import RangeSlider from "svelte-range-slider-pips";
  import { Alert } from "flowbite-svelte";
  import { by } from "../lib/mapable";
  import { Activity } from "../lib/activity";
  import { Usage } from "../lib/usage";
  import { parts } from "../lib/part";
  import { category } from "../lib/types";
  import ActName from "./ActName.svelte";

  export let acts: Activity[];

  let selection: Record<string | number, any> = {};

  let min: number,
    max: number,
    values: number[] = [];

  MiniMax(0);

  function MiniMax(gear: Number) {
    let set = acts
      .filter((a) => !gear || a.gear == gear)
      .map((a) => a.start.getTime() / DAY);
    max = Math.floor(
      set.reduce((res, start) => (start > res ? start : res), 0),
    );
    min = Math.floor(
      set.reduce((res, start) => (start < res ? start : res), max),
    );
    values = [min, max];
  }

  $: MiniMax(selection.gear);

  $: rows = filterRows(acts, values);

  function filterRows(acts: Activity[], values: number[]) {
    let start = new Date(values[0] * DAY);
    let end = new Date(values[1] * DAY);

    return acts.filter((a) => {
      let starttime = a.start.getTime();
      return start.getTime() <= starttime && end.getTime() + DAY > starttime;
    });
  }

  const formatter = (v: number) => new Date(v * DAY).toLocaleDateString();

  const createFilterOptions = (acts: Activity[]) => {
    let types: any = {};
    // we assume acts is sorted by start. Maybe wrongly so?
    acts.forEach((act) => {
      if (types[act.gear || 0] === undefined) {
        if (act.gear) {
          let name = act.gearName($parts);
          types[act.gear] = { name: name, value: act.gear, start: act.start };
        } else {
          types[0] = { name: "-- none --", value: 0, start: new Date() };
        }
      }
    });
    let res = Object.values(types).sort(by<any>("start"));

    return res.length > 1 ? res : undefined;
  };

  $: columns = [
    {
      key: "start",
      title: "Start",
      value: (v: Activity) => v.start,
      sortable: true,
      searchValue: (v: Activity) =>
        v.start.toLocaleDateString() + " " + v.start.toLocaleTimeString(),
      renderValue: (v: Activity) =>
        v.start
          ? v.start.toLocaleDateString() +
            "&nbsp;" +
            v.start.toLocaleTimeString()
          : "",
      totalsValue: () => "Total:",
      parseHTML: true,
    },
    {
      key: "name",
      title: "Name",
      value: (v: Activity) => v.name || "",
      searchValue: (v: Activity) => v.name,
      sortable: true,
      renderComponent: ActName,
      totalsValue: (a: Activity) => a.count + " activities",
      parseHTML: true,
    },
    {
      key: "gear",
      title: $category.name,
      renderValue: (a: any) => new Activity(a).gearLink($parts),
      totalsValue: () => "",
      parseHTML: true,
      sortable: true,
      filterPlaceholder: "All",
      filterValue: (a: Activity, f: any) => f === (a.gear ? a.gear : 0),
      filterOptions: createFilterOptions(acts),
    },
    {
      key: "time",
      title: "Time",
      value: (v: Activity) => v.time,
      renderValue: (v: Activity) => fmtSeconds(v.time),
      sortable: true,
      class: "text-end",
    },
    {
      key: "distance",
      title: "Distance",
      value: (v: Activity) => v.distance,
      renderValue: (v: Activity) => fmtNumber(v.distance),
      sortable: true,
      class: "text-end",
    },
    {
      key: "climb",
      title: "Climb",
      value: (v: Activity) => v.climb,
      renderValue: (v: Activity) => fmtNumber(v.climb),
      sortable: true,
      class: "text-end",
    },
    {
      key: "descend",
      title: "Descend",
      value: (v: Activity) => v.descend || v.climb,
      renderValue: (v: Activity) => fmtNumber(v.descend || v.climb),
      sortable: true,
      class: "text-end text-wrap",
    },
    {
      key: "energy",
      title: "Energy",
      value: (v: Activity) => v.energy,
      renderValue: (v: Activity) => fmtNumber(v.energy),
      sortable: true,
      class: "text-end",
    },
    {
      key: "device_name",
      title: "Data Source",
      value: (a: Activity) => a.device_name || "",
      searchValue: (v: Activity) => v.device_name,
      sortable: true,
      parseHTML: true,
    },
  ];

  const totalsFunc = (r: Usage[]) => {
    return r.reduce((total, row) => {
      total.add(row);
      return total;
    }, new Usage());
  };
</script>

{#if acts.length == 0}
  <Alert color="secondary">No activities</Alert>
{:else}
  <div class="mb-20">
    <RangeSlider
      {min}
      {max}
      range
      pushy
      pips
      first="label"
      last="label"
      float
      {formatter}
      bind:values
    ></RangeSlider>
  </div>

  <SvelteTable
    {columns}
    {rows}
    sortOrders={[-1, 1]}
    sortBy="start"
    {totalsFunc}
    bind:filterSelections={selection}
    classNameSelect="w-auto"
    classNameInput="p-1 dark:bg-gray-500 bg-gray-200"
  />
{/if}

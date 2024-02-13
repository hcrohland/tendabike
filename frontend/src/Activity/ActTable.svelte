<script lang="ts">
  import { DAY, fmtNumber, fmtSeconds } from "../lib/store";
  import SvelteTable from "../Widgets/SvelteTable.svelte";
  import RangeSlider from "svelte-range-slider-pips";
  import { Alert } from "@sveltestrap/sveltestrap";
  import { by } from "../lib/mapable";
  import { Activity } from "./activity";
  import { Usage } from "../Usage/usage";
  import { parts } from "../Part/part";

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
    acts.forEach((act) => {
      if (act.gear && types[act.gear] === undefined) {
        let name = act.gearName($parts);
        types[act.gear] = { name: name, value: act.gear };
      }
    });
    let res = Object.values(types).sort(by<any>("value"));
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
      renderValue: (v: Activity) =>
        v.id
          ? '<a href="/strava/activities/' +
            v.id +
            '" style="text-decoration:none" class="text-reset" target="_blank">' +
            v.name +
            '&nbsp;&nbsp;<img src="strava_grey.png" alt="View on Strava" title="View on Strava" />'
          : v.name,
      totalsValue: (a: Activity) => a.count + " activities",
      parseHTML: true,
    },
    {
      key: "gear",
      title: "Gear",
      renderValue: (a: any) => new Activity(a).gearLink($parts),
      totalsValue: () => "",
      parseHTML: true,
      sortable: true,
      filterValue: (v: Activity) => v.gear,
      filterOptions: createFilterOptions(acts),
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
      key: "time",
      title: "Time",
      value: (v: Activity) => v.time,
      renderValue: (v: Activity) => fmtSeconds(v.time),
      sortable: true,
      class: "text-end",
    },
    {
      key: "duration",
      title: "Duration",
      value: (v: Activity) => v.duration,
      renderValue: (v: Activity) => fmtSeconds(v.duration),
      sortable: true,
      class: "text-end",
    },
  ];

  const totalsFunc = (r: Usage[]) => {
    return r.reduce((total, row) => {
      total.add(row);
      return total;
    }, new Usage());
  };
</script>

{#if rows.length == 0}
  <Alert color="secondary" heading="No activities" />
{:else}
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

  <SvelteTable
    {columns}
    {rows}
    sortOrders={[-1, 1]}
    sortBy="start"
    {totalsFunc}
    bind:filterSelections={selection}
    classNameTable="table"
    classNameThead="table-secondary"
    classNameSelect="custom-select"
    classNameInput="form-control form-control-sm"
  />
{/if}

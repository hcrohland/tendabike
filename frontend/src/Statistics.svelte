<script lang="ts">
  import { category } from "./lib/types";
  import {
    ButtonGroup,
    InputAddon,
    MultiSelect,
    Select,
    TabItem,
    Tabs,
  } from "flowbite-svelte";
  import Plotly from "./Widgets/Plotly.svelte";
  import Switch from "./Widgets/Switch.svelte";
  import { by } from "./lib/mapable";
  import { Activity, activities } from "./lib/activity";
  import { Usage } from "./lib/usage";
  import { parts, Part } from "./lib/part";

  class Day extends Usage {
    start: Date;
    constructor(a: Activity | Date | Day) {
      if (a instanceof Date) {
        super();
        this.start = new Date(a);
      } else if (a instanceof Activity) {
        let b = {
          count: a.count || 0,
          distance: (a.distance || 0) / 1000,
          time: (a.time || a.duration || 0) / 3600,
          duration: (a.duration || 0) / 3600,
          descend: a.descend || a.climb || 0,
          climb: a.climb || 0,
        };
        super(b);
        this.start = new Date(a.start);
      } else {
        super(a);
        this.start = new Date(a.start);
      }
    }
  }

  type Year = {
    year: number;
    days: Day[];
    months: Day[];
  };

  function sumByMonths(arr: Day[]) {
    return Object.values(
      arr.reduce<{ [s: string]: Day }>((acc, a: Day) => {
        let start = new Date(a.start);
        start.setHours(0, 0, 0, 0);
        start.setDate(13);
        let diy = start.toString();
        if (!acc[diy]) acc[diy] = new Day(start);
        acc[diy].add(a);
        return acc;
      }, {}),
    );
  }

  /// Build a timeline with accumulated Usage data for every activity in arr
  function aggregateDays(arr: Day[]): Day[] {
    if (arr.length < 2) return arr;

    let start = arr.sort(by("start", true)).shift() as Day;
    return arr.reduce(
      (r, a) => {
        let b = new Day(a); // do not modify arr
        b.add(r[r.length - 1]);
        r.push(b);
        return r;
      },
      [start],
    );
  }

  function buildYears(acts1: Activity[], gear: Part[]): Year[] {
    const g = gear.map((g) => g.id);

    const minyear = acts1
      .reduce((min, a) => (min <= a.start ? min : a.start), new Date())
      .getFullYear();
    const thisyear = new Date().getFullYear();
    let ret = [];
    let year: number;
    for (year = thisyear; year >= minyear; year--) {
      // get a copy of all bike activities for year year
      let acts = acts1
        .filter(
          (a) =>
            a.start.getFullYear() == year &&
            (g.length == 0 || g.includes(a.gear)),
        )
        .map((a) => new Day(a));
      ret.push({
        year,
        days: aggregateDays(acts),
        months: sumByMonths(acts),
      });
    }
    return ret;
  }

  function get_trace(
    cum: Day[],
    months: boolean,
    field: keyof Usage,
    title?: string,
    field2?: keyof Usage,
  ) {
    return {
      x: cum.map((a) => a.start),
      y: cum.map((a) =>
        field2
          ? (a[field] as number) - (a[field2] as number)
          : (a[field] as number),
      ),
      type: months ? "bar" : "scatter",
      name: title ? title : field2 ? field + "-" + field2 : field,
      line: { dash: "solid", shape: "hv" },
      opacity: 1,
    };
  }

  function getPlot(
    _trigger: any,
    ncumm: number,
    ncomp: number | null,
    months: boolean,
    title: string,
    fields: string[][],
    addlayout?: any,
  ) {
    const colorway2 = ["steelblue", "lightblue", "limegreen", "lightgreen"];
    const colorway = [colorway2[0], colorway2[2]];

    let data = [];
    let layout = {
      title: { text: title },
      legend: { orientation: "h" },
      yaxis: {
        hoverformat: ".4r",
        fixedrange: true,
        rangemode: "tozero",
      },
      xaxis: {
        tickformat: "%b",
        dtick: 30 * 24 * 60 * 60 * 1000, // One month
        hoverformat: "%e %b",
        fixedrange: true,
        range: [
          new Date(years[0].year, 0, 1),
          new Date(years[0].year, 11, 31, 23, 59),
        ],
        anchor: "y0",
      },
      hovermode: "x",
      annotations: [],
      colorway,
      ...addlayout,
    };

    let yanchor = "middle";
    for (const field of fields.values()) {
      for (const [indx, y] of [ncumm, ncomp].entries()) {
        if (y == undefined) break;
        let year = years[y];
        let trace = get_trace(
          months ? year.months : year.days,
          months,
          field[0] as keyof Usage,
          field[1],
        );
        trace.x.map((a) => a.setFullYear(years[0].year));
        if (indx) {
          trace.line.dash = "dash";
          trace.name = trace.name + ` (${year.year})`;
          layout.colorway = colorway2;
        }
        data.push(trace);
      }
      yanchor = "bottom";
    }
    return {
      data,
      layout,
    };
  }

  let title = $state("elevation");

  function plot(
    title: string,
    perMonths: boolean,
    cumm: number,
    comp: number | null,
    years: Year[],
  ) {
    if (title == "time") {
      if (perMonths)
        return getPlot(
          years,
          cumm,
          null,
          perMonths,
          "Time (h)",
          [
            ["time", "moving time"],
            ["duration", "pause", "time"],
          ],
          { barmode: "stack" },
        );
      else
        return getPlot(years, cumm, comp, perMonths, "Time (h)", [
          ["time", "moving time"],
          ["duration", "outdoor time"],
        ]);
    } else if (title == "distance")
      return getPlot(years, cumm, comp, perMonths, "Distance (km)", [
        ["distance"],
      ]);
    else
      return getPlot(years, cumm, comp, perMonths, "Elevation (m)", [
        ["climb"],
        ["descend"],
      ]);
  }

  let acts = $derived($category.activities($activities));
  let gears = $derived($category.parts($parts));
  let gear = $state([]);
  let cumm: any = $state(0);
  let comp: number | null = $state(null);
  let perMonths = $state(false);
  let years = $derived(buildYears(acts, gear));
</script>

<div class="flex flex-wrap pb-10 gap-2 justify-between">
  <ButtonGroup>
    <InputAddon>Your statistics for</InputAddon>
    <Select
      bind:value={cumm}
      onchange={() => {
        if (cumm == comp) comp = null;
      }}
      classes={{ select: "rounded-none h-full" }}
    >
      {#each years as item, i}
        <option value={i}>{item.year}</option>
      {/each}
    </Select>
    <InputAddon>vs</InputAddon>
    <Select bind:value={comp} classes={{ select: "rounded-none h-full" }}>
      {#each years as item, i (item.year)}
        {@const selected = (comp ? comp : cumm) == i}
        {#if i != cumm}
          <option value={i} {selected}>{item.year}</option>
        {:else}
          <option value={null} {selected}>-- None --</option>
        {/if}
      {/each}
    </Select>
    <InputAddon>
      <Switch bind:checked={perMonths} class="">Per Month</Switch>
    </InputAddon>
  </ButtonGroup>
  <MultiSelect
    placeholder="Select bikes..."
    items={gears
      .sort(by("last_used"))
      .map((g, n) => ({ value: g, name: n + " - " + g.name, id: g.id }))}
    bind:value={gear}
    class="max-w-150"
  />
</div>

<Tabs bind:selected={title}>
  <TabItem title="Elevation" key="elevation" />
  <TabItem title="Distance" key="distance" />
  <TabItem title="Time" key="time" />
</Tabs>
<Plotly {...plot(title, perMonths, cumm!, comp, years)} />

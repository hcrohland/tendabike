<script lang="ts">
  import { category } from "./lib/store";
  import {
    Row,
    Col,
    Input,
    InputGroup,
    InputGroupText,
    TabPane,
    TabContent,
  } from "@sveltestrap/sveltestrap";
  import Plotly from "./Widgets/Plotly.svelte";
  import Switch from "./Widgets/Switch.svelte";
  import { by } from "./lib/mapable";
  import { Activity, activities } from "./Activity/activity";
  import { Usage } from "./Usage/usage";
  import { parts, Part } from "./Part/part";

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
    ncumm: Year,
    ncomp: Year | null,
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
        hoverformat: ".3r",
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
      annotations: [],
      colorway,
      ...addlayout,
    };

    let yanchor = "middle";
    for (const field of fields.values()) {
      for (const [indx, year] of [ncumm, ncomp].entries()) {
        if (year == null) break;
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

        if (!months) {
          let ann = trace.y[trace.y.length - 1];
          let result2 = {
            x: trace.x[trace.x.length - 1],
            y: ann,
            xanchor: "left",
            yanchor,
            text: Math.round(ann),
            showarrow: false,
          };

          layout.annotations.push(result2);
        }
      }
      yanchor = "bottom";
    }
    return {
      data,
      layout,
    };
  }

  let tab = "elevation";

  function plot(
    tab: string,
    perMonths: boolean,
    cumm: Year,
    comp: Year | null,
  ) {
    if (tab == "time") {
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
    } else if (tab == "distance")
      return getPlot(years, cumm, comp, perMonths, "Distance (km)", [
        ["distance"],
      ]);
    else
      return getPlot(years, cumm, comp, perMonths, "Elevation (m)", [
        ["climb"],
        ["descend"],
      ]);
  }

  $: acts = $category.activities($activities);
  $: gears = $category.parts($parts);
  let gear = gears ? [...gears] : [];
  let cumm: Year;
  let comp: Year | null = null;
  let perMonths = false;
  $: years = buildYears(acts, gear);
</script>

<Row class="p-sm-2">
  <Col xs="auto" class="p-0 p-sm-2">
    <InputGroup>
      <InputGroupText>Your statistics for</InputGroupText>
      <Input
        type="select"
        class="custom-select"
        bind:value={cumm}
        on:change={() => {
          if (cumm.year == comp?.year) comp = null;
        }}
      >
        {#each years as item}
          <option value={item}>{item.year}</option>
        {/each}
      </Input>
      <InputGroupText>vs</InputGroupText>
      <Input type="select" class="custom-select" bind:value={comp}>
        {#each years as item (item.year)}
          {@const selected = (comp ? comp.year : cumm?.year) == item.year}
          {#if item.year != cumm?.year}
            <option value={item} {selected}>{item.year}</option>
          {:else}
            <option value={null} {selected}>-- None --</option>
          {/if}
        {/each}
      </Input>
      <Switch id="months" bind:checked={perMonths}>Per Month</Switch>
    </InputGroup>
  </Col>
  <Col class="p-0 p-sm-2" />
  <Col xs="auto" class="p-0 p-sm-2">
    <InputGroup>
      <select
        class="form-select"
        multiple
        placeholder="Select bikes..."
        bind:value={gear}
      >
        {#each gears as item, i}
          <option value={item}>{item.name}</option>
        {/each}
      </select>
    </InputGroup>
  </Col>
</Row>
<TabContent on:tab={(e) => (tab = e.detail.toString())}>
  <TabPane tab="Elevation" tabId="elevation" active />
  <TabPane tab="Distance" tabId="distance" />
  <TabPane tab="Time" tabId="time" />
</TabContent>
<Plotly {...plot(tab, perMonths, cumm, comp)} />

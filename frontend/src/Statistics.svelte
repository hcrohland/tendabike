<script lang="ts">
import {activities, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupAddon, InputGroupText} from 'sveltestrap'
import type { Usage, Activity } from './types';
import { addToUsage, newUsage } from './types';
import Plotly from './Widgets/Plotly.svelte';
import Switch from './Widgets/Switch.svelte';


type Day = Usage & {
  start: Date,
}

function sumByMonths (arr: Day[]) {
  return Object.values(
    arr.reduce<{ [s: string]: Day; }>((acc, a: Activity) => {
      let start = new Date (a.start)
      start.setHours(0,0,0,0);
      start.setDate(13)
      let diy = start.toString()
      if (!acc[diy]) acc[diy] = {start, ...newUsage() };
      addToUsage(acc[diy], a);
      return acc
      }, {})
  )
}

/// Build a timeline with accumulated Usage data for every activity in arr 
function aggregateDays (arr: Day[]) : Day[]{
  if (arr[0] === undefined) return;
  let start = {start: arr[0].start, ...newUsage()};
  return arr
    .sort(by("start", true))
    .reduce(
      function(r, a) {
        addToUsage( a, r[r.length - 1]);
        r.push(a); 
        return r;
      }, 
      [start]
    )
    
}

// create a new - human readable - day out of an activity
function activity2Day (a: Activity) : Day {
  return {
      start: a.start,
      count: a.count,
      distance: a.distance / 1000,
      time: (a.time ?  a.time : a.duration) / 3600,
      duration:  a.duration / 3600,
      descend: a.descend ? a.descend : a.climb,
      climb: a.climb
    }
}

type Year = {year: number, days: Day[], months: Day[]};
function buildYears():Year[] {
  let ret = [];
  let minyear = Object.values($activities)
    .reduce((min, a) =>  min <= a.start ? min : a.start, new Date())
    .getFullYear()
  let thisyear = new Date().getFullYear();
  let year: number;
  for (year = thisyear; year >= minyear; year--) {
    // get a copy of all bike activities for year year
    let acts = filterValues($activities, (a) => a.start.getFullYear() == year && a.what == 1)
      // and translate usage data to human readable form
      .map(activity2Day)
      
    ret.push(
      {
        year, 
        days: aggregateDays (acts),
        months: sumByMonths (acts)
      } 
    )
  }
  return ret; 
}

function get_trace (cum: Day[], months: boolean, field: keyof Usage, title?: string, field2?: keyof Usage) {
  return {
    x: cum.map((a)=>a.start),
    y: cum.map((a)=>a[field]-(field2? a[field2] : 0)),
    type: months ? 'bar' : 'scatter',
    name: title ? title : field2? field + '-' + field2 : field,
    line: {dash: 'solid', shape: 'hv'},
    opacity: 1,
  }
}

function getPlot(years: Year[], months, title, fields, addlayout?) {
  let data = [];
  let layout =  {
    title: {text: title},
    legend:{"orientation": "h"},
    yaxis: {
      hoverformat: '.3r',
      fixedrange: true,
    },
    xaxis: {
      tickformat: '%b',
      dtick: 30 * 24 * 60 * 60 * 1000,
      hoverformat: '%e %b',
      fixedrange: true,
      range: [new Date (years[0].year, 0, 1), new Date(years[0].year, 11, 31, 23, 59)]
    },
    annotations: []
  };
  Object.assign(layout, addlayout);
  let config = {responsive: true};
  
  let comp = null
  for (let year of years) {
    if (!year) break;
    fields.forEach(f => {
      let d = get_trace(months ? year.months : year.days, months, f[0], f[1]);
      if (comp) {
        d.x.map((a) => a.setFullYear(comp));
        d.line.dash = 'dash';
        d.opacity = 0.5;
        d.name = d.name + ` (${year.year})`;
      }
      data.push(d);
      
      if (!months) {
        let ann = d.y[d.y.length-1];
        let result2 = {
          x: d.x[d.x.length-1],
          y: ann,
          xanchor: 'left',
          yanchor: 'middle',
          text: Math.round(ann),
          showarrow: false
        };
        
        layout.annotations.push( result2);
      }
    });
    comp = year.year
  }
  return {
    data, config, layout
  }
}

let years = buildYears();
let comp
let cumm=years[0];
let perMonths = false;
</script>
<Row border class="p-sm-2">
  <Col xs="auto" class="p-0 p-sm-2">
  <FormGroup inline>
    <InputGroup>
      <InputGroupAddon addonType="prepend">
        <InputGroupText>
          Your statistics for
        </InputGroupText>
      </InputGroupAddon>
      <select class="custom-select" bind:value={cumm}>
        {#each years as item, i}
          <option value={item}>{item.year}</option>
        {/each}
      </select>
      <InputGroupAddon addonType="prepend">
        <InputGroupText>
          vs
        </InputGroupText>
      </InputGroupAddon>
      <select class="custom-select" bind:value={comp}>
        {#each years as item, i}
          {#if item != cumm}
            <option value={item}>{item.year}</option>
          {:else}
            <option value={null}>-- None --</option>
          {/if}
        {/each}
      </select>
      <Switch id="months" bind:checked={perMonths}>
        Per Month
      </Switch>
    </InputGroup>
  </FormGroup>
</Col>
</Row>
<Row border class="p-sm-2">
  <Col class="p-0 p-sm-2">
    <Plotly {...getPlot([cumm, comp], perMonths, "Elevation (m)", [["climb"], ["descend"]])}  />
  </Col>
</Row>
<Row>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly {...getPlot([cumm, comp], perMonths, "Distance (km)", [["distance"]])} />
  </Col>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    {#if perMonths}
      <Plotly {...getPlot([cumm], perMonths, "Time (h)", [[ "time", "moving time"], ["duration", "pause", "time"]], {barmode: 'stack'})} />
    {:else}
      <Plotly {...getPlot([cumm, comp], perMonths, "Time (h)", [[ "time", "moving time"], ["duration", "outdoor time"]])} />
    {/if}
  </Col>
</Row>
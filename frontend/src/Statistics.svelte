<script lang="ts">
import {activities, parts, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupText, NavItem} from 'sveltestrap'
import type { Part, Usage, Activity } from './types';
import { addToUsage, newUsage } from './types';
import Plotly from './Widgets/Plotly.svelte';
import Switch from './Widgets/Switch.svelte';

type Day = Usage & {
  start: Date,
}

type Year = {
  year: number, 
  days: Day[], 
  months: Day[]
};

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
  if (arr[0] === undefined) return [];
  let start = {start: arr[0].start, ...newUsage()};
  return arr
    .sort(by("start", true))
    .reduce(
      function(r, a) {
        let b = {...a}; // do not modify arr
        addToUsage(b , r[r.length - 1]);
        r.push(b); 
        return r;
      }, 
      [start]
    )
    
}

// create a new - human readable - day out of an activity
function activity2Day (a: Activity) : Day {
  return {
      start: new Date(a.start),
      count: a.count,
      distance: a.distance / 1000,
      time: (a.time ?  a.time : a.duration) / 3600,
      duration:  a.duration / 3600,
      descend: a.descend ? a.descend : a.climb,
      climb: a.climb
    }
}

function buildYears(select: Boolean, gear: Part[]):Year[] {
  const g = gear.map((g) => g.id);
  const minyear = Object.values($activities)
    .reduce((min, a) =>  min <= a.start ? min : a.start, new Date())
    .getFullYear()
  const thisyear = new Date().getFullYear();
  let ret = [];
  let year: number;
  for (year = thisyear; year >= minyear; year--) {
    // get a copy of all bike activities for year year
    let acts = filterValues($activities, (a) => a.start.getFullYear() == year && a.what == 1 && (select == false || g.includes(a.gear)))
      // and translate usage data to human readable form
      .map(activity2Day)
      
    ret.push(
      {
        year, 
        days: aggregateDays (acts),
        months: sumByMonths (acts),
      } 
    )
  }
  return ret; 
}

function get_trace (cum: Day[], months: boolean, field: keyof Usage, title?: string, field2?: keyof Usage) {
  return {
    x: cum.map( (a) => a.start ),
    y: cum.map( (a) => field2 ? a[field]-a[field2] : a[field] ),
    type: months ? 'bar' : 'scatter',
    name: title ? title : field2? field + '-' + field2 : field,
    line: {dash: 'solid', shape: 'hv'},
    opacity: 1,
  }
}

function getPlot(_trigger, ncumm: number, ncomp: number, months, title, fields, addlayout?) {
  const colorway2 = ['steelblue', 'lightblue', 'limegreen', 'lightgreen'] 
  const colorway = [colorway2[0], colorway2[2]] 

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
      dtick: 30 * 24 * 60 * 60 * 1000, // One month
      hoverformat: '%e %b',
      fixedrange: true,
      range: [new Date (years[0].year, 0, 1), new Date(years[0].year, 11, 31, 23, 59)]
    },
    annotations: [],
    colorway,
    ...addlayout
  };
  let config = {responsive: true};
  
  let yanchor = 'middle'
  for (const field of fields.values()) {
    for (const [indx, y] of [ncumm, ncomp].entries()) {
      if (y == undefined) break;
      let year = years[y];
      let trace = get_trace(months ? year.months : year.days, months, field[0], field[1]);
      trace.x.map((a) => a.setFullYear(years[0].year));
      if (indx) {
        trace.line.dash = 'dash';
        trace.name = trace.name + ` (${year.year})`;
        layout.colorway = colorway2;
      }
      data.push(trace);
      
      if (!months) {
        let ann = trace.y[trace.y.length-1];
        let result2 = {
          x: trace.x[trace.x.length-1],
          y: ann,
          xanchor: 'left',
          yanchor,
          text: Math.round(ann),
          showarrow: false
        };
        
        layout.annotations.push( result2);
      }
    }
    yanchor = 'bottom'

  };
  return {
    data, config, layout
  }
}

const gears = filterValues($parts, (p) => p.what == 1).sort(by("last_used"))

let selectBikes = false;
let gear = [...gears];
let cumm = 0;
let comp = null;
let perMonths = false;
$: years = buildYears(selectBikes, gear);
</script>
<Row border class="p-sm-2">
  <Col xs="auto" class="p-0 p-sm-2">
  <FormGroup inline>
    <InputGroup>
      <InputGroupText>
        Your statistics for
      </InputGroupText>
      <select class="custom-select" bind:value={cumm} on:change={() => {if (cumm == comp) comp = null}}>
        {#each years as item, i}
          <option value={i}>{item.year}</option>
        {/each}
      </select>
      <InputGroupText>
        vs
      </InputGroupText>
      <select class="custom-select" bind:value={comp}>
        {#each years as item, i}
          {#if i != cumm}
            <option value={i}>{item.year}</option>
          {:else}
            <option value={null} selected>-- None --</option>
          {/if}
        {/each}
      </select>
      <Switch id="months" bind:checked={perMonths}>
        Per Month
      </Switch>
    </InputGroup>
    </FormGroup>
  </Col>
  <Col class="p-0 p-sm-2" />
  <Col xs="auto" class="p-0 p-sm-2" float="right">
    <FormGroup inline>      
      <InputGroup>
        <Switch id="bikes" bind:checked={selectBikes}>
          {selectBikes ? "for" : "select"} bikes
        </Switch>
        {#if selectBikes}
           <select class="custom-select" multiple bind:value={gear}>
            {#each gears as item, i}
            <option value={item}>{item.name}</option>
            {/each}
          </select>
        {/if}
      </InputGroup>
    </FormGroup>
  </Col>
</Row>
<Row border class="p-sm-2">
  <Col class="p-0 p-sm-2">
    <Plotly {...getPlot(years, cumm, comp, perMonths, "Elevation (m)", [["climb"], ["descend"]])}  />
  </Col>
</Row>
<Row>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly {...getPlot(years, cumm, comp, perMonths, "Distance (km)", [["distance"]])}/>
  </Col>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    {#if perMonths}
      <Plotly {...getPlot(years, cumm, null, perMonths, "Time (h)", [[ "time", "moving time"], ["duration", "pause", "time"]], {barmode: 'stack'})} />
    {:else}
      <Plotly {...getPlot(years, cumm, comp, perMonths, "Time (h)", [[ "time", "moving time"], ["duration", "outdoor time"]])} />
    {/if}
  </Col>
</Row>
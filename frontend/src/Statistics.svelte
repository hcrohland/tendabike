<script lang="ts">
import {activities, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupAddon, InputGroupText} from 'sveltestrap'
import type { Usage, Activity } from './types';
import { addToUsage, newUsage } from './types';
import Plotly from './Widgets/Plotly.svelte';
import ActivityList from './Widgets/ActivityList.svelte';


type Day = Usage & {
  start: Date,
}

/// Build a timeline with accumulated Usage data for every activity in arr 
function sumUp (arr: Day[]) : Day[]{
  if (arr[0] === undefined) return;
  let start = newUsage() as Day;
  start.start = arr[0].start;
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

type Year = {year: number, data: Day[]};
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
        data: sumUp (acts)
      } 
    )
  }
  return ret; 
}

function get_trace (cum: Day[], field: keyof Usage, title?: string) {
  let name = title ? title : field
  return {
    x: cum.map((a)=>a.start),
    y: cum.map((a)=>a[field]),
    type: 'scatter',
    name,
    line: {dash: 'solid', shape: 'hv'},
    opacity: 1,
  }
}

function getPlot(years: Year[], title, fields, addlayout?) {

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
      let d = get_trace(year.data, f[0], f[1]);
      if (comp) {
        d.x.map((a) => a.setFullYear(comp));
        d.line.dash = 'dash';
        d.opacity = 0.5;
        d.name = d.name + ` (${year.year})`;
      }
      data.push(d);
      
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
    </InputGroup>
  </FormGroup>
</Col>
</Row>
<Row border class="p-sm-2">
  <Col class="p-0 p-sm-2">
    <Plotly {...getPlot([cumm, comp], "Elevation (m)", [["climb"], ["descend"]])}  />
  </Col>
</Row>
<Row>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly {...getPlot([cumm, comp], "Distance (km)", [["distance"]])} />
  </Col>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly {...getPlot([cumm, comp], "Time (h)", [[ "time", "moving time"], ["duration", "outdoor time"]])} />
  </Col>
</Row>
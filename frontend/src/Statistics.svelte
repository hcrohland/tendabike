<script lang="ts">
import {activities, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupAddon, InputGroupText} from 'sveltestrap'
import type { Usage, Activity } from './types';
import Plotly from './Widgets/Plotly.svelte';
import Switch from './Widgets/Switch.svelte'
export let year = new Date().getFullYear();

let months = false;

function getday(date){
    var day = new Date(date)
    day.setHours(0,0,0,0);
    return day
}

function getmonth(date) {
  let month = getday(date)
  month.setDate(1)
  return month
}

type Day = {
  date: Date,
  acts: Activity[],
  distance: number,
  climb: number,
  descend: number,
  duration: number,
  time: number
}

let minyear = new Date(Object.values($activities).sort(by("start")).pop().start).getFullYear()
let thisyear = new Date().getFullYear()

function get_cum(year: number, months: Boolean){
  let acts = filterValues($activities, (a) => new Date(a.start).getFullYear() == year && a.what == 1)
      .sort(by("start", true))

  let days = Object.values(
    acts.reduce<{ [s: string]: Day; }>((acc, a: Activity) => {
      let date = months ? getmonth (a.start) : getday (a.start)
      let diy = date.toString()
      if (!acc[diy]) {
        acc[diy] = {date, acts:[],climb: 0,descend: 0, time :0, duration:0, distance:0}
      }
      acc[diy].acts.push(a)
      acc[diy].distance += a.distance / 1000
      acc[diy].climb += a.climb
      acc[diy].descend += a.descend ? a.descend : a.climb
      acc[diy].duration += a.duration /3600
      acc[diy].time += (a.time ? a.time : a.duration) /3600

      return acc
      }, {})
  )
  
  if (months) return days

  return days.reduce<Day[]>(function(r, a) {
                    if (r.length > 0){
                      a.distance += r[r.length - 1].distance;
                      a.climb += r[r.length - 1].climb;
                      a.descend += r[r.length - 1].descend;
                      a.duration += r[r.length - 1].duration;
                      a.time += r[r.length - 1].time;
                    }
                    r.push(a);
                    return r;
                  }, []);
}

function get_trace (cum: Day[], field: keyof Usage, title?: string, field2?: keyof Usage) {
  return {
    x: cum.map((a)=>a.date),
    y: cum.map((a)=>a[field]-(field2? a[field2] : 0)),
    type: months ? 'bar' : 'scatter',
    name: title ? title : field2? field + '-' + field2 : field,
    line: {shape: 'hv'},
  }
}

$: cummulative = get_cum(year, months); 

let layout =  {showlegend:true, legend:{"orientation": "h"},
xaxis: {
      tickformat: months ? '%b %y' : undefined
}}
let config = {responsive: true}
            
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
      <select class="custom-select" bind:value={year}>
        {#each Array(thisyear-minyear) as item, i}
        <option value={thisyear-i}>{thisyear-i}</option>
        {/each}
      </select>
      <Switch id="dispose" bind:checked={months}>
        Per Month
      </Switch>
    </InputGroup>
  </FormGroup>
</Col>
</Row>
<Row border class="p-sm-2">
  <Col class="p-0 p-sm-2">
    <Plotly title="Elevation (m)" data={[get_trace(cummulative, "climb"),get_trace(cummulative, "descend")]} {layout} {config} />
  </Col>
</Row>
<Row>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly title="Distance (km)" data={[get_trace(cummulative, "distance" )]} {layout} {config} />
  </Col>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    {#if months}
       <Plotly 
          title="Time (h)" 
          data={[get_trace(cummulative, "time", "moving time"),get_trace(cummulative, "duration", "pause", "time")]} 
          layout={{legend:{"orientation": "h"},barmode: 'stack'}} 
          {config} />
    {:else}
       <Plotly 
          title="Time (h)" 
          data={[get_trace(cummulative, "time", "moving time"),get_trace(cummulative, "duration", "outdoor time")]} 
          {layout} 
          {config} />
    {/if}
  </Col>
</Row>
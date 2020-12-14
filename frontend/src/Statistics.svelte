<script lang="ts">
import {activities, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupAddon, InputGroupText} from 'sveltestrap'
import type { Usage, Activity } from './types';
import Plotly from './Widgets/Plotly.svelte';
export let year = new Date().getFullYear();

function daysIntoYear(day){
    var date = new Date(day)
    return (Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) - Date.UTC(date.getFullYear(), 0, 0)) / 24 / 60 / 60 / 1000;
}

type Day = {
  day: number,
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

function get_cum(year){
  let acts = filterValues($activities, (a) => new Date(a.start).getFullYear() == year && a.what == 1)
      .sort(by("start", true))

  let days = Object.values(
    acts.reduce<{ [s: string]: Day; }>((acc, a: Activity) => {
      var diy = daysIntoYear (a.start)
      if (!acc[diy]) {
        acc[diy] = {day: diy, date: new Date(a.start), acts:[],climb: 0,descend: 0, time :0, duration:0, distance:0}
      }
      acc[diy].day = diy
      acc[diy].acts.push(a)
      acc[diy].distance += a.distance / 1000
      acc[diy].climb += a.climb
      acc[diy].descend += a.descend ? a.descend : a.climb
      acc[diy].duration += a.duration /3600
      acc[diy].time += (a.time ? a.time : a.duration) /3600

      return acc
      }, {})
  )

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

function get_trace (cum: Day[], field: keyof Usage, field2?: keyof Usage) {
  return {
    x: cum.map((a)=>a.date),
    y: cum.map((a)=>a[field]-(field2? a[field2] : 0)),
    type: 'scatter',
    name: field2? field + '-' + field2 : field,
    line: {shape: 'hv'},
  }
}

$: cummulative = get_cum(year)

let layout =  {showlegend:true, legend:{"orientation": "h"}}
let config = {responsive: true}
            
</script>
<Row border class="p-sm-2">
  <Col xs=3 class="p-0 p-sm-2">
  <FormGroup inline>
    <InputGroup>
      <InputGroupAddon addonType="prepend">
        <InputGroupText>
          Your statistics for
        </InputGroupText>
      </InputGroupAddon>
      <select class="custom-select" bind:value={year}>
        {#each Array(thisyear-minyear) as item, i}
        <!-- content here -->
        <option value={thisyear-i}>{thisyear-i}</option>
        {/each}
      </select>
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
    <Plotly title="Time (h)" data={[get_trace(cummulative, "time"),get_trace(cummulative, "time", "duration")]} {layout} {config} />
  </Col>
</Row>
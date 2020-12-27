<script lang="ts">
import {activities, by, filterValues} from './store'
import {Row, Col, FormGroup, InputGroup, InputGroupAddon, InputGroupText} from 'sveltestrap'
import type { Usage, Activity } from './types';
import Plotly from './Widgets/Plotly.svelte';
import Switch from './Widgets/Switch.svelte'
import _ from 'lodash';

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
  start: Date,
  distance: number,
  climb: number,
  descend: number,
  duration: number,
  time: number
}

function groupByMonth (arr: Activity[]) {
  return Object.values(
    arr.reduce<{ [s: string]: Day; }>((acc, a: Activity) => {
      let start = months ? getmonth (a.start) : getday (a.start)
      start.setDate(20)
      let diy = start.toString()
      if (!acc[diy]) {
        acc[diy] = {start, climb: 0,descend: 0, time :0, duration:0, distance:0}
      }
      acc[diy].distance += a.distance
      acc[diy].climb += a.climb
      acc[diy].descend += a.descend ? a.descend : a.climb
      acc[diy].duration += a.duration
      acc[diy].time += (a.time ? a.time : a.duration)

      return acc
      }, {})
  )
}

function sumUp (arr: Activity[]) {
  return arr.reduce(function(r, a) {
                    if (r.length > 0){
                      a.distance  += r[r.length - 1].distance;
                      a.descend   += r[r.length - 1].descend;
                      a.climb     += r[r.length - 1].climb;
                      a.time      += r[r.length - 1].time;
                      a.duration  += r[r.length - 1].duration;
                    }
                    r.push(a);
                    return r;
                  }, []);
}

let minyear = new Date(Object.values($activities).sort(by("start")).pop().start).getFullYear()
let thisyear = new Date().getFullYear()

function get_cum(year: number, months: Boolean){
  let acts = _.cloneDeep(filterValues($activities, (a) => new Date(a.start).getFullYear() == year && a.what == 1))
      .map((a) => {
        a.distance /= 1000;
        a.time = (a.time ?  a.time : a.duration) /3600;
        a.duration /= 3600;
        a.descend = a.descend ? a.descend : a.climb;
        return a
      })
      .sort(by("start", true))

  if (months)
    return groupByMonth(acts)
  else
    return sumUp(acts)
}

function get_trace (cum: Day[], field: keyof Usage, title?: string, field2?: keyof Usage) {
  return {
    x: cum.map((a)=>a.start),
    y: cum.map((a)=>a[field]-(field2? a[field2] : 0)),
    type: months ? 'bar' : 'scatter',
    name: title ? title : field2? field + '-' + field2 : field,
    line: {shape: 'hv'},
  }
}

function getPlot(cummulative, fields, addlayout?) {
  let data = [];
  let layout =  Object.assign({
    legend:{"orientation": "h"},
    yaxis: {hoverformat: '.3r'},
    xaxis: {
      tickformat: '%b',
      dtick: 30 * 24 * 60 * 60 * 1000,
      ticklabelposition: 'outside right'
    },
    annotations: []
  }, addlayout)
  let config = {responsive: true}
  
  fields.forEach(f => {
    let d = get_trace(cummulative, f[0], f[1], f[2]);
    data.push(d);
    
    if (!months){
      let ann = d.y[d.y.length-1];
      let result2 = {
      xref: 'paper',
      x: 1,
      y: ann,
      xanchor: 'left',
      yanchor: 'middle',
      text: Math.round(ann),
      showarrow: false
    };

    layout.annotations.push( result2);
  }
  });
  return {
    data,config, layout
  }
}
   
   
$: cumm = get_cum(year, months); 

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
        {#each Array(thisyear-minyear+1) as item, i}
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
    <Plotly title="Elevation (m)" {...getPlot(cumm, [["climb"], ["descend"]])}  />
  </Col>
</Row>
<Row>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    <Plotly title="Distance (km)" {...getPlot(cumm, [["distance"]])} />
  </Col>
  <Col md=6 xs=12 class="p-0 p-sm-2">
    {#if months}
      <Plotly title="Time (h)" {...getPlot(cumm, [[ "time", "moving time"], ["duration", "pause", "time"]], {barmode: 'stack'})} />
    {:else}
      <Plotly title="Time (h)" {...getPlot(cumm, [[ "time", "moving time"], ["duration", "outdoor time"]])} />
    {/if}
  </Col>
</Row>
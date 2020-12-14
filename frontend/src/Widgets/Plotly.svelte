<script>
  import { onMount } from 'svelte';
  export let data;
  export let layout = undefined;
  export let config = undefined;
  
  let redraw = (a,b,c) => {};

  let div = 'plotly_div_' + Math.random()

  onMount(() => {
  
      let plotDiv = document.getElementById(div)

      Plotly.newPlot(plotDiv, data, layout, config); 
      redraw = (d, l, c) => Plotly.react(plotDiv,d, l, c) 
  });
  
  $: redraw(data, layout, config)
</script>

  <div id={div}><!-- Plotly chart will be drawn inside this DIV --></div>
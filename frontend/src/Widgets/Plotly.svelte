<script>
  // @ts-nocheck
  import { onMount } from 'svelte';
  export let data;
  export let layout = undefined;
  export let config = undefined;

  let redraw = (a,b,c) => {};

  let plotDiv


  onMount(() => {
      redraw = (d, l, c) => Plotly.newPlot(plotDiv,d, l, c) 
  });
  
  $: redraw(data, layout, config)
</script>

  <div bind:this={plotDiv}><!-- Plotly chart will be drawn inside this DIV --></div>
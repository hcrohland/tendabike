<script>
  import { onMount } from 'svelte';
  export let data;
  export let layout = undefined;
  export let config = undefined;
  export let title

  let redraw = (a,b,c) => {};

  let plotDiv

  layout = Object.assign({}, layout, {title: {text: title}})

  onMount(() => {
      redraw = (d, l, c) => Plotly.newPlot(plotDiv,d, l, c) 
  });
  
  $: redraw(data, layout, config)
</script>

  <div bind:this={plotDiv}><!-- Plotly chart will be drawn inside this DIV --></div>
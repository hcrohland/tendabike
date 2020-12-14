<script>
  import { onMount } from 'svelte';
  export let data;
  export let layout = undefined;
  export let config = undefined;
  export let title

  let redraw = (a,b,c) => {};

  let div = 'plotly_div_' + title

  layout = Object.assign(Object.assign({}, layout), {title: {text: title}})
  onMount(() => {
  
      let plotDiv = document.getElementById(div)

      Plotly.newPlot(plotDiv, data, layout, config); 
      redraw = (d, l, c) => Plotly.newPlot(plotDiv,d, l, c) 
  });
  
  $: redraw(data, layout, config)
</script>

  <div id={div}><!-- Plotly chart will be drawn inside this DIV --></div>
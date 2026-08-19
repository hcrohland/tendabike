<script lang="ts">
  let { data, layout }: { data?: any; layout?: any } = $props();

  let config = { responsive: true };

  let plotDiv: HTMLDivElement;

  $effect(() => {
    if (plotDiv && typeof (window as any).Plotly !== "undefined") {
      (window as any).Plotly.newPlot(plotDiv, data, layout, config);
    }
  });
</script>

<div bind:this={plotDiv}>
  <!-- Plotly chart will be drawn inside this DIV -->
</div>

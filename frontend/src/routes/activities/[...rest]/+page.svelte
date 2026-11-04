<script lang="ts">
  import { page } from "$app/state";
  import Activities from "../../../Activity/Activities.svelte";

  const segments = $derived((page.params.rest ?? "").split("/"));
  const part = $derived(segments[0] ? Number(segments[0]) : 0);
  const start = $derived(segments[1] ? Number(segments[1]) : undefined);
</script>

<Activities params={{ part, start }} />

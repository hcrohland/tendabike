<script lang="ts">
  import { page } from "$app/state";
  import Activities from "../../../../Activity/Activities.svelte";
</script>

<Activities
  params={{
    part: page.params.part ? Number(page.params.part) : 0,
    start: page.params.start ? Number(page.params.start) : undefined,
  }}
/>

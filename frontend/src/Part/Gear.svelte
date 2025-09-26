<script lang="ts">
  import Part from "./Part.svelte";

  export let params: { id: number };
</script>

<Part {...params} />

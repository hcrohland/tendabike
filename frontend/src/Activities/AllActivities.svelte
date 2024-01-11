<script lang="ts">
  import { activities, category } from "../lib/store";
  import ActTable from "./ActTable.svelte";
</script>

<ActTable acts={$category.activities($activities)} />
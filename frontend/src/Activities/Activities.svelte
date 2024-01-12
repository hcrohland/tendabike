<script lang="ts">
  import { activities, attachments, category, filterValues, parts } from "../lib/store";
  import { Activity } from "../lib/types";
  import ActTable from "./ActTable.svelte";

  export let params: { part: number; start?: number };

  let acts: Activity[];
  $: if (params.part) {
    let part = $parts[params.part];
    if (part.isGear()) {
      acts = filterValues($activities, (a) => a.gear == part.id)
    } else {
      let start = params.start;
      let atts = part
        .attachments($attachments)
        .filter((a) => (start ? a.isAttached(start) : true));
      acts = atts.map((att) => att.activities($activities)).flat();
    }
  } else {
    acts = $category.activities($activities);
  }
</script>

<ActTable {acts} />

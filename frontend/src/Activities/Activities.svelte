<script lang="ts">
  import {
    activities,
    attachments,
    category,
    parts,
  } from "../lib/store";
  import { Activity } from "../lib/types";
  import ActTable from "./ActTable.svelte";

  export let params: { part: number; start?: number }

  let acts: Activity[];
  $: if (params.part) {
    let start = params.start;
    let atts = $parts[params.part]
      .attachments($attachments)
      .filter((a) => (start ? a.isAttached(start) : true));
    console.log(atts);

    acts = atts
      .map((att) => att.activities($activities))
      .flat();
  } else {
    acts = $category.activities($activities);
  }
</script>

<ActTable {acts} />

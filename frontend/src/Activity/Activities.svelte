<script lang="ts">
  import { Card, CardHeader } from "@sveltestrap/sveltestrap";
  import { category } from "../lib/types";
  import { Activity, activities } from "../lib/activity";
  import ActTable from "./ActTable.svelte";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";

  export let params: { part: number; start?: number };

  let acts: Activity[];
  let title: string;
  $: if (params.part) {
    let part = $parts[params.part];
    title = " for " + part.name;
    if (part.isGear()) {
      acts = filterValues($activities, (a) => a.gear == part.id);
    } else {
      let start = Number(params.start);
      let atts = part
        .attachments($attachments)
        .filter((a) => (start ? a.isAttached(start) : true));
      acts = atts.map((att) => att.activities($activities)).flat();
      if (start)
        title =
          title +
          " attached to " +
          ($parts[atts[0].gear] ? $parts[atts[0].gear].name : "unknown part") +
          " since " +
          atts[0].fmtTime();
    }
  } else {
    title = "";
    acts = $category.activities($activities);
  }
</script>

<Card>
  <CardHeader class="text-center h5 mb-0" color="secondary">
    All activities {title}
  </CardHeader>
</Card>
<ActTable {acts} />

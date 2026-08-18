<script lang="ts">
  import { Card } from "flowbite-svelte";
  import { category } from "../lib/types";
  import { Activity, activities } from "../lib/activity";
  import ActList from "./ActList.svelte";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import * as m from "../../paraglide/messages";

  export let params: { part: number; start?: number };

  let acts: Activity[];
  let title: string;
  $: if (params.part) {
    let part = $parts[params.part];
    title = m.act_heading_for({ name: part.name });
    if (part.isGear()) {
      acts = filterValues($activities, (a) => a.gear == part.id);
    } else {
      let start = Number(params.start);
      let atts = part
        .attachments($attachments)
        .filter((a) => (start ? a.isAttached(start) : true));
      acts = atts.map((att) => att.activities($activities)).flat();
      if (start)
        title = m.act_heading_attached({
          name: part.name,
          part: $parts[atts[0].gear]
            ? $parts[atts[0].gear].name
            : m.act_unknown_part(),
          date: atts[0].fmtTime(),
        });
    }
  } else {
    title = m.act_heading_all();
    acts = $category.activities($activities);
  }
</script>

<div class="flex justify-around">
  <Card class="flex text-xl text-center p-4 mb-4">
    {title}
  </Card>
</div>
<ActList {acts} />

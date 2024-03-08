<script lang="ts">
  import GearCard from "./GearCard.svelte";
  import { Part } from "../lib/part";
  import { category } from "../lib/store";
  import { link } from "svelte-spa-router";

  export let part: Part;
  export let display: boolean;
</script>

<GearCard {part} {display}>
  <div class="float-end">
    <a
      href="/part/{part.id}"
      use:link
      class="badge badge-secondary text-decoration-none"
      title={"View " + $category.name.toLowerCase() + " details"}
    >
      &Longrightarrow;
    </a>
  </div>
</GearCard>

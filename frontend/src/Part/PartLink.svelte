<script lang="ts">
  import { link } from "svelte-spa-router";
  import { Part } from "../lib/part";

  export let part: Part;
</script>

<a
  href="/part/{part.id}"
  use:link
  style={part.disposed_at ? "text-decoration: line-through;" : ""}
  class="text-reset"
>
  {part.name}
</a>

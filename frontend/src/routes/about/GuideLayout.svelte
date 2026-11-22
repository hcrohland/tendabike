<script lang="ts">
  import Sidebar from "./Sidebar.svelte";

  let { children } = $props();
</script>

<!-- The sidebar is a flex sibling of the content column. On /about the
     provider connect cards sit above this row at full page width, so the
     sidebar never covers them. -->
<div class="flex flex-col gap-4 md:flex-row md:gap-8">
  <Sidebar />
  <div class="min-w-0 flex-1">
    {@render children()}
  </div>
</div>

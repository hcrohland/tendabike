<script lang="ts">
  import { page } from "$app/state";
  import ContentPage from "../../../lib/ContentPage.svelte";
</script>

<ContentPage page="about/{page.params.page}" />

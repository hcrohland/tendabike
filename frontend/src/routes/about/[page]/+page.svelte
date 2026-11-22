<script lang="ts">
  import { page } from "$app/state";
  import ContentPage from "../../../lib/ContentPage.svelte";
  import GuideLayout from "../GuideLayout.svelte";
</script>

<GuideLayout>
  <ContentPage page="about/{page.params.page}" />
</GuideLayout>

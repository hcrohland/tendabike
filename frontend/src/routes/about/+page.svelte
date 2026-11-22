<script lang="ts">
  import ContentPage from "../../lib/ContentPage.svelte";
  import ConnectProviders from "../../Widgets/ConnectProviders.svelte";
  import { getUser } from "../../lib/user.svelte";
  import GuideLayout from "./GuideLayout.svelte";
</script>

<!-- The connect screen (the provider hero cards) is the landing content for
     anonymous visitors: the layout redirects unauthenticated sessions here.
     It sits above the guide row at full page width. -->
{#if !getUser()}
  <ConnectProviders />
{/if}
<GuideLayout>
  <ContentPage page="about" />
</GuideLayout>

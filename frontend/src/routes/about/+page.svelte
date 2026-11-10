<script lang="ts">
  import ContentPage from "../../lib/ContentPage.svelte";
  import ConnectProviders from "../../Widgets/ConnectProviders.svelte";
  import { getUser } from "../../lib/user";
</script>

<!-- The connect screen (the provider hero cards) is the landing content for
     anonymous visitors: the layout redirects unauthenticated sessions here. -->
{#if !getUser()}
  <ConnectProviders />
{/if}
<ContentPage page="about" />

<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";
  import { ThemeProvider } from "flowbite-svelte";
  import Header from "../Header.svelte";
  import OfflineBanner from "../OfflineBanner.svelte";
  import NotificationToasts from "../Widgets/NotificationToasts.svelte";
  import InitialSyncDialog from "../Widgets/InitialSyncDialog.svelte";
  import Actions from "../Widgets/Actions.svelte";
  import ShopFrame from "../Shop/ShopFrame.svelte";
  import Footer from "../Footer.svelte";
  import { loadProviders } from "../lib/provider.svelte";

  // The enabled providers are app-wide state: the layout
  // fetches them once so the offline banner and the connect screen read
  // the same module state.
  onMount(() => {
    loadProviders();
  });
</script>

<div class="max-w-4xl">
  <Header />
  <OfflineBanner />
  <NotificationToasts />
  <InitialSyncDialog />
  <ThemeProvider>
    <div class="m-1 md:m-8">
      <ShopFrame>
        <slot />
      </ShopFrame>
    </div>
  </ThemeProvider>
  <Actions />
  <Footer />
</div>

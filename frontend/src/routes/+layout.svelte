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
  import { loadProviders } from "../lib/provider";

  // The enabled providers are app-wide state (ticket #925): the layout
  // fetches them once so the offline banner and the connect screen read
  // the same module state.
  onMount(() => {
    loadProviders();
  });

  const theme = {
    tableBodyCell: "px-2 py-3",
    tableHeadCell: "px-2 py-3",
  };
</script>

<div class="max-w-4xl">
  <Header />
  <OfflineBanner />
  <NotificationToasts />
  <InitialSyncDialog />
  <ThemeProvider {theme}>
    <div class="m-1 md:m-8">
      <ShopFrame>
        <slot />
      </ShopFrame>
    </div>
  </ThemeProvider>
  <Actions />
  <Footer />
</div>

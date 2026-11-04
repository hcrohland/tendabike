<script lang="ts">
  import "../app.css";
  import { ThemeProvider } from "flowbite-svelte";
  import Header from "../Header.svelte";
  import NotificationToasts from "../Widgets/NotificationToasts.svelte";
  import InitialSyncDialog from "../Widgets/InitialSyncDialog.svelte";
  import Actions from "../Widgets/Actions.svelte";
  import ShopFrame from "../Shop/ShopFrame.svelte";

  const theme = {
    tableBodyCell: "px-2 py-3",
    tableHeadCell: "px-2 py-3",
  };
</script>

<div class="max-w-4xl">
  <Header />
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
</div>

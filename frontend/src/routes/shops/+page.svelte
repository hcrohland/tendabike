<script lang="ts">
  import { page } from "$app/state";
  import Shops from "../../Shop/Shops.svelte";
</script>

<Shops
  query={{ tab: new URLSearchParams(page.url.search).get("tab") ?? undefined }}
/>

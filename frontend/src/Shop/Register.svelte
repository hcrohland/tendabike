<script lang="ts">
  import Subscriptions from "./Subscriptions.svelte";

  export let params: { shopid: number };
</script>

<Subscriptions shopid={params.shopid} />

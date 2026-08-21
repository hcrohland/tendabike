<script lang="ts">
  import Subscriptions from "./Subscriptions.svelte";

  let { params }: { params: { shopid: number } } = $props();
</script>

<Subscriptions shopid={params.shopid} />

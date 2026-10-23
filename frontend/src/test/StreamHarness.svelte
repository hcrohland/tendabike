<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    markHydrated,
    startStream,
    stopStream,
    type EventSourceFactory,
  } from "../lib/stream";
  import { getUser } from "../lib/user";
  import { getShop } from "../lib/shop";

  /// A test seam mirroring Header's stream effect: the stream opens once the
  /// user is known, and re-opens (closing the previous source) whenever the
  /// stream target changes — a stable string key derived from user + shop,
  /// so `setUser()` from a refresh does not re-open the stream.
  let {
    promise,
    factory,
  }: {
    promise: Promise<void>;
    factory: EventSourceFactory;
  } = $props();

  let streamKey = $derived.by(() => {
    if (!getUser()) return null;
    const shop = getShop();
    return shop ? "shop:" + shop.id : "personal";
  });

  $effect(() => {
    promise.then(
      () => markHydrated(),
      () => markHydrated(),
    );
    if (streamKey !== null) {
      startStream(() => {}, factory);
    }
  });

  onDestroy(() => stopStream());
</script>

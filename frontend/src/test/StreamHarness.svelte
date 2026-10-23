<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    markHydrated,
    startStream,
    stopStream,
    type EventSourceFactory,
  } from "../lib/stream";
  import { getUser } from "../lib/user";

  /// A test seam mirroring Header's stream effect: the stream opens once the
  /// user is known, and re-opens (closing the previous source) whenever
  /// `getShop()` changes — `startStream` reads the shop at open time, so the
  /// effect tracks it. The `promise` is the initData catch-up snapshot.
  let {
    promise,
    factory,
  }: {
    promise: Promise<void>;
    factory: EventSourceFactory;
  } = $props();

  $effect(() => {
    promise.then(
      () => markHydrated(),
      () => markHydrated(),
    );
    if (getUser()) {
      startStream(() => {}, factory);
    }
  });

  onDestroy(() => stopStream());
</script>

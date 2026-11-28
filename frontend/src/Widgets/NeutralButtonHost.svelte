<!-- Test host: renders a NeutralButton in a given configuration so a .ts
     test (which cannot build snippet children) can observe the rendered
     fill role class. -->
<script lang="ts">
  import NeutralButton from "./NeutralButton.svelte";

  let {
    mode = "button",
  }: {
    mode?: "button" | "link" | "submit";
  } = $props();
</script>

{#if mode === "button"}
  <NeutralButton onclick={() => {}}>label</NeutralButton>
{:else if mode === "link"}
  <NeutralButton href="/somewhere">label</NeutralButton>
{:else}
  <NeutralButton type="submit" value="commit">label</NeutralButton>
{/if}

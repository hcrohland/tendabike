<!-- Test host: renders a Card in a given configuration so a .ts test (which
     cannot build snippet children) can observe the tone flip and context. -->
<script lang="ts">
  import Card from "./Card.svelte";
  import Tone from "./Tone.svelte";

  let {
    mode = "default",
  }: {
    mode?: "default" | "light" | "nested" | "tone";
  } = $props();
</script>

{#if mode === "default"}
  <Card>
    <span>hello</span>
  </Card>
{:else if mode === "light"}
  <Card light>
    <span>hello</span>
  </Card>
{:else if mode === "nested"}
  <Card>
    <Card>
      <span>nested</span>
    </Card>
  </Card>
{:else if mode === "tone"}
  <Card>
    <Tone>content</Tone>
  </Card>
{/if}

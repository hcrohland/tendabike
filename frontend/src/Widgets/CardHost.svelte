<!-- Test host: renders a Card (and a nested Card / Tone block) in a given
     configuration so a .ts test (which cannot build snippet children) can
     observe the rendered surface/fill classes. -->
<script lang="ts">
  import Card from "./Card.svelte";
  import Tone from "./Tone.svelte";

  let {
    mode = "default",
  }: {
    mode?:
      | "default"
      | "nested"
      | "color-primary"
      | "color-secondary"
      | "tone-color-primary"
      | "tone-color-secondary";
  } = $props();
</script>

{#if mode === "default"}
  <Card>
    <span>hello</span>
  </Card>
{:else if mode === "nested"}
  <Card>
    <Card color="primary">
      <span>nested</span>
    </Card>
  </Card>
{:else if mode === "color-primary"}
  <Card color="primary">
    <span>hello</span>
  </Card>
{:else if mode === "color-secondary"}
  <Card color="secondary">
    <span>hello</span>
  </Card>
{:else if mode === "tone-color-primary"}
  <Card>
    <Tone color="primary">content</Tone>
  </Card>
{:else if mode === "tone-color-secondary"}
  <Card>
    <Tone color="secondary">content</Tone>
  </Card>
{/if}

<!-- Test host: renders a Card in a given configuration so a .ts test (which
     cannot build snippet children) can observe the tone flip and context. -->
<script lang="ts">
  import Card from "./Card.svelte";
  import Tone from "./Tone.svelte";

  let {
    mode = "default",
  }: {
    mode?:
      | "default"
      | "light"
      | "nested"
      | "tone"
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

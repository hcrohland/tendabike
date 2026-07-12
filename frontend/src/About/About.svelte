<script lang="ts">
  import { getLocale } from "../../paraglide/runtime";
  import AboutEn from "./AboutContent.en.svelte";
  import AboutDe from "./AboutContent.de.svelte";
</script>

{#if getLocale() === "de"}
  <AboutDe />
{:else}
  <AboutEn />
{/if}

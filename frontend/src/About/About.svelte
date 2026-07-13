<script lang="ts">
  import { getLocale } from "../../paraglide/runtime";
  import AboutEn from "./AboutContent.en.svelte";
  import AboutDe from "./AboutContent.de.svelte";
  import AboutDeCH from "./AboutContent.de_CH.svelte";
</script>

{#if getLocale() === "de"}
  <AboutDe />
{:else if getLocale() === "de_CH"}
  <AboutDeCH />
{:else}
  <AboutEn />
{/if}

<!--
	PROTOTYPE — throwaway, do not ship. See ChipFill.svelte for the question
	and variants. This is the app-route host: it reads ?variant= and hands it
	to the shared prototype component.
-->

<script lang="ts">
  import { page } from "$app/state";
  import ChipFill from "./ChipFill.svelte";

  const initial = page.url.searchParams.get("variant") ?? "B";
</script>

<ChipFill initialVariant={initial} />

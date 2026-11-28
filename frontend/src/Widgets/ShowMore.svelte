<script lang="ts">
  import { Tooltip } from "flowbite-svelte";
  import { ClockOutline, CloseCircleOutline } from "flowbite-svelte-icons";
  import { m } from "../../paraglide/messages";
  import NeutralButton from "./NeutralButton.svelte";

  interface Props {
    show_more: boolean;
    title?: string | undefined;
    update?: (v: boolean) => void;
  }

  let {
    show_more = $bindable(false),
    title = undefined,
    update = (_) => {},
  }: Props = $props();

  function onclick() {
    show_more = !show_more;
    update(show_more);
  }
</script>

<NeutralButton size="sm" class="rounded-full mx-1" {onclick}>
  {#if show_more}
    <CloseCircleOutline class="shrink-0 h-4 w-4" />
    {#if title}
      <Tooltip placement="top" class=" text-xs p-1 rounded-xs">
        {m.showmore_hide({ title })}
      </Tooltip>
    {/if}
  {:else}
    <ClockOutline class="shrink-0 h-4 w-4" />
    {#if title}
      <Tooltip placement="top" class=" text-xs p-1 rounded-xs">
        {m.showmore_show({ title })}
      </Tooltip>
    {/if}
  {/if}
</NeutralButton>

<script lang="ts">
  import { Button, Tooltip } from "flowbite-svelte";
  import { CircleMinusOutline, CirclePlusOutline } from "flowbite-svelte-icons";
  interface Props {
    show_more: boolean;
    title?: string | undefined;
    update?: (v: boolean) => void;
  }

  let {
    show_more = $bindable(false),
    title = undefined,
    update = (v) => {},
  }: Props = $props();

  function onclick() {
    show_more = !show_more;
    update(show_more);
  }

  let delay = 1000;
</script>

<Button
  size="sm"
  color="alternative"
  class="align-middle justify-center rounded-full border-0 p-0 m-0 mx-1"
  {onclick}
>
  {#if show_more}
    <CircleMinusOutline class="shrink-0 h-5 w-5" />
    {#if title}
      <Tooltip placement="top" class=" text-xs p-1 rounded-xs">
        Hide {title}
      </Tooltip>
    {/if}
  {:else}
    <CirclePlusOutline class="shrink-0 h-5 w-5" />
    {#if title}
      <Tooltip placement="top" class=" text-xs p-1 rounded-xs">
        Show {title}
      </Tooltip>
    {/if}
  {/if}
</Button>

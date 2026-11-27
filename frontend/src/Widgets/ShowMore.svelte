<script lang="ts">
  import { Button, Tooltip } from "flowbite-svelte";
  import { ClockOutline, CloseCircleOutline } from "flowbite-svelte-icons";
  import { m } from "../../paraglide/messages";
  import { fillBg, type Color } from "./color.js";

  interface Props {
    show_more: boolean;
    title?: string | undefined;
    update?: (v: boolean) => void;
    color?: Color;
  }

  let {
    show_more = $bindable(false),
    title = undefined,
    update = (_) => {},
    color = "secondary",
  }: Props = $props();

  function onclick() {
    show_more = !show_more;
    update(show_more);
  }
</script>

<Button
  size="sm"
  {color}
  class={"align-middle justify-center rounded-full border-0 p-0 m-0 mx-1 " +
    fillBg}
  {onclick}
>
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
</Button>

<script lang="ts">
  import { Card, Badge } from "flowbite-svelte";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import { user, users } from "../lib/user";
  import UsageChips from "../Usage/UsageChips.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
    summary?: boolean;
    gridclass?: string;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, gridclass, children }: Props = $props();

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return m.gearcard_unknown_model();
    } else {
      return part.vendor + " " + part.model;
    }
  }

  function typeName(part: Part) {
    if (part.what != types[part.what].main) {
      return types[part.what].localizedName();
    }
    return "";
  }
</script>

<Card size="xl" class="col-auto relative">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={"text-xl bg-surface-2 p-2 sm:p-4" +
      (summary
        ? " hover:bg-gray-300 dark:hover:bg-gray-500 cursor-pointer"
        : "")}
    onclick={() => summary && push("/part/" + part.id)}
  >
    {#if part.owner == $user!.id}
      {#if summary}
        <a href="/part/{part.id}" use:link class="text-decoration-none">
          {part.name}
        </a>
      {:else}
        {part.name}
      {/if}
    {:else}
      {@const owner = $users[part.owner]}
      {part.name}
      <Badge color="green" class="relative -top-3 -right-1">
        {owner.firstname + " " + owner.name}
      </Badge>
    {/if}
    {@render children?.()}
  </div>

  <div class="p-2 sm:p-4 bg-surface-1">
    <!-- Meta line: model · type · date range -->
    <p class="text-sm mb-3">
      {model(part)}{typeName(part) ? " · " + typeName(part) : ""}
      {#if part.what == 1}
        <a href={"/strava/bikes/" + part.id} target="_blank">
          <img
            src="strava_grey.png"
            alt={m.gearcard_view_on_strava()}
            title={m.gearcard_view_on_strava()}
            class="inline ml-1"
          />
        </a>
      {/if}
      <span class="text-text-1">
        ·
        {#if !part.disposed_at}
          {m.time_since()} {fmtDate(part.purchase)}
        {:else}
          {fmtDate(part.purchase)} – {fmtDate(part.disposed_at)}
        {/if}
      </span>
    </p>

    <!-- Stat chips -->
    <UsageChips id={part.usage} ref={part.id} {gridclass} light />
  </div>
</Card>

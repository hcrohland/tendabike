<script lang="ts">
  import { Card, Badge } from "flowbite-svelte";
  import { goto } from "$app/navigation";
  import { Part } from "../lib/part";
  import { fmtDate } from "../lib/utils";
  import { types } from "../lib/types.svelte";
  import { getUser, users } from "../lib/user.svelte";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ProviderMark from "../Widgets/ProviderMark.svelte";
  import { colorClasses, mutedText } from "../Widgets/color.js";
  import { providerOfActivityId, providerMeta } from "../lib/provider.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
    summary?: boolean;
    gridclass?: string;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, gridclass, children }: Props = $props();

  let gearProvider = $derived(
    part.source ? providerOfActivityId(part.source) : undefined,
  );
  // The provider-neutral gear redirect route: the provider is the part's
  // own, derived server-side (the typed absence — icu gear, a manual part —
  // is a no-op redirect to the current page).
  let gearUrl = $derived(part.source ? "/source/gear/" + part.id : undefined);

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
    class={"text-xl " +
      colorClasses["secondary"].surface +
      " p-2 sm:p-4" +
      (summary
        ? " hover:bg-gray-300 dark:hover:bg-gray-500 cursor-pointer"
        : "")}
    onclick={() => summary && goto("/part/" + part.id)}
  >
    {#if part.owner == getUser()!.id}
      {#if summary}
        <a href="/part/{part.id}" class="text-decoration-none">
          {part.name}
        </a>
      {:else}
        {part.name}
      {/if}
    {:else}
      {@const owner = users[part.owner]}
      {part.name}
      <Badge color="gray" class="relative -top-3 -right-1">
        {owner.firstname + " " + owner.name}
      </Badge>
    {/if}
    {@render children?.()}
  </div>

  <div class={"p-2 sm:p-4 " + colorClasses["primary"].surface}>
    <!-- Meta line: model · type · date range -->
    <p class="text-sm mb-3 flex flex-wrap items-center gap-x-1.5">
      <span>
        {model(part)}{typeName(part) ? " · " + typeName(part) : ""}
      </span>
      {#if gearProvider && gearUrl}
        <a
          href={gearUrl}
          target="_blank"
          class="inline-flex items-center"
          title={m.gearcard_view_on({
            provider: providerMeta[gearProvider].name,
          })}
        >
          <ProviderMark provider={gearProvider} size="sm" />
        </a>
      {/if}
      <span class={mutedText}>
        ·
        {#if !part.disposed_at}
          {m.time_since()} {fmtDate(part.purchase)}
        {:else}
          {fmtDate(part.purchase)} – {fmtDate(part.disposed_at)}
        {/if}
      </span>
    </p>

    <!-- Stat chips -->
    <UsageChips id={part.usage} ref={part.id} {gridclass} />
  </div>
</Card>

<script lang="ts">
  import { Card, Textarea, Button, Badge } from "flowbite-svelte";
  import { EditOutline } from "flowbite-svelte-icons";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import { fmtDate, handleError } from "../lib/store";
  import { types } from "../lib/types";
  import { user, users } from "../lib/user";
  import UsageChips from "../Usage/UsageChips.svelte";
  import ServiceBadge from "../Widgets/ServiceBadge.svelte";
  import * as m from "../../paraglide/messages";

  interface Props {
    part: Part;
    summary?: boolean;
    dues?: any;
    gridclass?: string;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, dues, gridclass, children }: Props = $props();

  let editingNotes = $state(false);
  let notesValue = $state("");

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return m.gearcard_unknown_model();
    } else {
      return part.vendor + " " + part.model;
    }
  }

  function typeName(part: Part) {
    if (part.what != types[part.what].main) {
      return types[part.what].name.toLowerCase();
    }
    return "";
  }

  function startEditNotes() {
    notesValue = part.notes;
    editingNotes = true;
  }

  function cancelEditNotes() {
    editingNotes = false;
    notesValue = "";
  }

  async function saveNotes() {
    try {
      const updatedPart = new Part({ ...part, notes: notesValue });
      await updatedPart.update();
      editingNotes = false;
    } catch (e: any) {
      handleError(e);
    }
  }
</script>

<Card size="xl" class="col-auto relative">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={"text-xl bg-surface-2 p-4" +
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

  <div class="p-4 bg-surface-1">
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
        <ServiceBadge service={dues?.days} />
      </span>
    </p>

    <!-- Stat chips -->
    <UsageChips id={part.usage} ref={part.id} {gridclass} {dues} light />

    <!-- Notes (detail view only) -->
    {#if !summary}
      <div class="mt-4">
        <div class="flex items-center gap-2 mb-2">
          <strong>{m.gearcard_notes()}:</strong>
          {#if !editingNotes}
            <EditOutline
              class="w-4 h-4 cursor-pointer text-gray-400 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
              onclick={startEditNotes}
            />
          {/if}
        </div>
        {#if editingNotes}
          <Textarea
            bind:value={notesValue}
            placeholder={m.gearcard_notes_placeholder()}
            rows={3}
            class="mb-2 w-full"
          />
          <div class="flex gap-2">
            <Button size="sm" onclick={saveNotes}>{m.gearcard_save()}</Button>
            <Button size="sm" color="alternative" onclick={cancelEditNotes}>
              {m.gearcard_cancel()}
            </Button>
          </div>
        {:else if part.notes}
          <p class="text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
            {part.notes}
          </p>
        {/if}
      </div>
    {/if}
  </div>
</Card>

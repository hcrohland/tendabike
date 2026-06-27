<script lang="ts">
  import { Card, Textarea, Button, Indicator, Tooltip } from "flowbite-svelte";
  import { EditOutline } from "flowbite-svelte-icons";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import { fmtDate, handleError } from "../lib/store";
  import { types } from "../lib/types";
  import { user, users } from "../lib/user";
  import UsageChips from "../Usage/UsageChips.svelte";

  interface Props {
    part: Part;
    summary?: boolean;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, children }: Props = $props();

  let editingNotes = $state(false);
  let notesValue = $state("");

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return "unknown model";
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
    class={"text-xl bg-gray-200 dark:bg-gray-700 p-4" +
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
      <Indicator placement="top-left">
        <Tooltip>
          {owner.firstname + " " + owner.name}
        </Tooltip>
      </Indicator>
    {/if}
    <div class="float-end h6 mb-0">
      {@render children?.()}
    </div>
  </div>

  <div class="p-4">
    <!-- Meta line: model · type · date range -->
    <p class="text-sm text-gray-500 dark:text-gray-400 mb-3">
      {model(part)}{typeName(part) ? " · " + typeName(part) : ""}
      {#if part.what == 1}
        <a href={"/strava/bikes/" + part.id} target="_blank">
          <img
            src="strava_grey.png"
            alt="View on Strava"
            title="View on Strava"
            class="inline ml-1"
          />
        </a>
      {/if}
      ·
      {#if !part.disposed_at}
        since {fmtDate(part.purchase)}
      {:else}
        {fmtDate(part.purchase)} – {fmtDate(part.disposed_at)}
      {/if}
    </p>

    <!-- Stat chips -->
    <UsageChips id={part.usage} ref={part.id} gridclass="grid-cols-3" />

    <!-- Notes (detail view only) -->
    {#if !summary}
      <div class="mt-4">
        <div class="flex items-center gap-2 mb-2">
          <strong>Notes:</strong>
          {#if !editingNotes}
            <EditOutline
              class="w-4 h-4 cursor-pointer text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              onclick={startEditNotes}
            />
          {/if}
        </div>
        {#if editingNotes}
          <Textarea
            bind:value={notesValue}
            placeholder="Add any notes about this part..."
            rows={3}
            class="mb-2 w-full"
          />
          <div class="flex gap-2">
            <Button size="sm" onclick={saveNotes}>Save</Button>
            <Button size="sm" color="alternative" onclick={cancelEditNotes}
              >Cancel</Button
            >
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

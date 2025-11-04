<script lang="ts">
  import { Card, Badge, Dropdown } from "flowbite-svelte";
  import { DotsVerticalOutline } from "flowbite-svelte-icons";
  import type { Snippet } from "svelte";
  import type { Garage } from "../lib/garage";
  import { onMount, onDestroy } from "svelte";

  interface Props {
    garage: Garage;
    isOwner?: boolean;
    sub?: Snippet<[Garage]>;
    children?: Snippet;
  }

  let { sub, garage, isOwner = false, children }: Props = $props();

  // Get bikes registered to this garage
  let partsCount = $state(0);

  async function loadPartsCount() {
    if (garage.id && isOwner) {
      try {
        const parts = await garage.getParts();
        partsCount = parts.length;
      } catch (error) {
        console.error("Error loading garage parts:", error);
        // Silently fail - user might not have permission
      }
    }
  }

  function handleGarageUpdate(event: CustomEvent) {
    if (event.detail.garageId === garage.id) {
      loadPartsCount();
    }
  }

  // Load parts count on mount
  onMount(() => {
    loadPartsCount();
    // Listen for garage updates
    window.addEventListener(
      "garage-updated",
      handleGarageUpdate as EventListener,
    );
  });

  onDestroy(() => {
    window.removeEventListener(
      "garage-updated",
      handleGarageUpdate as EventListener,
    );
  });
</script>

<Card size="xl" class="relative col-auto p-4">
  {#if children}
    <div class="absolute top-4 right-4">
      <DotsVerticalOutline class="cursor-pointer" />
      <Dropdown>
        {@render children?.()}
      </Dropdown>
    </div>
  {/if}

  <div class="space-y-3">
    <div>
      <h5
        class="text-xl font-bold tracking-tight text-gray-900 dark:text-white"
      >
        {garage.name}
      </h5>
      {#if !isOwner && garage.owner_firstname && garage.owner_name}
        <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
          by {garage.owner_firstname}
          {garage.owner_name}
        </p>
      {/if}
      {#if garage.description}
        <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">
          {garage.description}
        </p>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if isOwner}
        <Badge color="blue"
          >{partsCount} {partsCount === 1 ? "bike" : "bikes"}</Badge
        >
        <Badge color="green">Owner</Badge>
      {/if}
    </div>

    <div class="text-xs text-gray-500 dark:text-gray-400">
      Created {new Date(garage.created_at).toLocaleDateString()}
    </div>
    {#if sub}
      {@render sub(garage)}
    {/if}
  </div>
</Card>

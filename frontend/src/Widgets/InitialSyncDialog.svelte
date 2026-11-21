<!--
	tendabike - the bike maintenance tracker

	Copyright (C) 2023  Christoph Rohland

	This program is free software: you can redistribute it and/or modify
	it under the terms of the GNU Affero General Public License as published
	by the Free Software Foundation, either version 3 of the License, or
	(at your option) any later version.

	This program is distributed in the hope that it will be useful,
	but WITHOUT ANY WARRANTY; without even the implied warranty of
	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
	GNU Affero General Public License for more details.

	You should have received a copy of the GNU Affero General Public License
	along with this program.  If not, see <https://www.gnu.org/licenses/>.

 -->
<script lang="ts">
  import { Modal, P, Heading, Button } from "flowbite-svelte";
  import { myfetch, handleError, icons } from "../lib/utils";
  import { getUser, setUser } from "../lib/user.svelte";
  import {
    providerMeta,
    onboardingGearUrl,
    onboardingSyncUrl,
    onboardingPostponeUrl,
    type Provider,
  } from "../lib/provider.svelte";
  import ProviderMark from "./ProviderMark.svelte";
  import { m } from "../../paraglide/messages";

  /// A gear item the onboarding gear import discovered (spec #829): the
  /// display name and the part type (the category — a bike, a shoe). The
  /// onboarding gear import's response lists them; the dialog presents them
  /// as chips.
  interface GearItem {
    name: string;
    what: number;
  }

  let open = $state(getUser()?.onboarding_status === "pending");

  /// The dialog is only open for a known user in pending status, so the
  /// provider (and with it the branding) is derived from the current user.
  /// The onboarding URLs are provider-neutral (spec #977).
  let provider = $derived((getUser()?.provider ?? "strava") as Provider);
  let meta = $derived(providerMeta[provider]);

  let loading = $state(false);

  /// The gear the onboarding import discovered, rendered as chips once the
  /// gear import's response lands.
  let gear = $state<GearItem[]>([]);
  let gearLoaded = $state(false);

  /// Step 1 (spec #829): on dialog open, import the gear from the provider
  /// and show the discovered items. No activity sync is queued — the user
  /// decides on that with the CTA.
  $effect(() => {
    if (open && !gearLoaded) {
      importGear();
    }
  });

  async function importGear() {
    loading = true;
    try {
      const res = await myfetch(onboardingGearUrl(), "POST");
      gear = res.gear ?? [];
      gearLoaded = true;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }

  /// Step 2 (spec #829): queue the activity sync and close the dialog.
  async function importActivities() {
    loading = true;
    try {
      const updatedUser = await myfetch(onboardingSyncUrl(), "POST");
      setUser(updatedUser);
      open = false;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }

  /// ESC on the welcome dialog (spec #974): the same effect as Skip for
  /// Now — record the postponement. The dialog stays open until the write
  /// lands (it closes on success), so a failed postponement cannot leave
  /// the dialog closed on a still-`pending` user (it would reopen on the
  /// next load).
  function onEscape(ev: Event) {
    if (loading) return;
    ev.preventDefault();
    skipSync();
  }

  /// Postpone the historic import (spec #974): the onboarding status
  /// becomes `initial_sync_postponed` — the import is postponed, not
  /// cancelled, and the provider-agnostic entry points (the top-bar
  /// Import Activities button, the user menu's Sync → Import Historic
  /// Activities) serve the user later.
  async function skipSync() {
    loading = true;
    try {
      const updatedUser = await myfetch(onboardingPostponeUrl(), "POST");
      setUser(updatedUser);
      open = false;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }
</script>

<Modal
  bind:open
  size="lg"
  autoclose={false}
  dismissable={false}
  outsideclose={false}
  oncancel={onEscape}
>
  <div
    class="rounded-lg border-2 overflow-hidden"
    style="border-color:{meta.accent}"
  >
    <div
      class="p-3 text-white flex items-center gap-2"
      style="background:{meta.accent}"
    >
      <ProviderMark {provider} colored />
      <span class="font-semibold">
        {m.initialsync_header({ provider: meta.name })}
      </span>
    </div>
    <div class="p-4 sm:p-6 bg-surface-1">
      <div class="text-center">
        <Heading tag="h3" class="mb-5 text-lg font-normal text-text-1">
          {m.initialsync_welcome()}
        </Heading>
        <P class="mb-4 text-left">
          {m.initialsync_question({ provider: meta.name })}
        </P>
        <P class="mb-6 text-left text-sm text-text-1">
          {m.initialsync_desc1()}
        </P>
        <P class="mb-6 text-left text-sm text-text-1">
          {m.initialsync_desc2()}
        </P>
        {#if provider === "strava"}
          <P class="mb-6 text-left text-sm text-text-1">
            {m.initialsync_desc3({
              sync: m.header_sync(),
              historic: m.header_import_historic(),
            })}
          </P>
        {/if}
        <P class="mb-6 text-left text-sm text-text-1">
          {m.initialsync_desc4()} <br />
        </P>
        {#if gear.length > 0}
          <!-- The gear the import discovered (spec #829): compact chips in a
               flex-wrap row, shown once the gear import's response lands. -->
          <div class="mb-6 text-left">
            <div
              class="text-xs font-medium text-text-1 uppercase tracking-wide mb-2"
            >
              {m.initialsync_gear()}
            </div>
            <div class="flex flex-wrap gap-2">
              {#each gear as item (item.name)}
                <span
                  class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm bg-surface-2 border border-border-subtle text-text-2"
                >
                  <!-- The part type's flaticon (mountain-bike for a bike,
                       run for a shoe, ...) via the shared icons map. -->
                  <i class={icons.get(item.what)}></i>
                  {item.name}
                </span>
              {/each}
            </div>
          </div>
        {/if}
        <!-- The CTA and the skip are visible alongside the gear
             chips: the user sees the imported gear and decides on the
             activity sync in one screen (spec #829). The skip is
             provider-agnostic (spec #974). -->
        <div class="flex justify-center gap-4">
          <Button
            color={provider === "strava" ? "orange" : "gray"}
            disabled={loading}
            onclick={importActivities}
          >
            {loading ? m.action_importing() : m.header_import_activities()}
          </Button>
          <Button color="alternative" disabled={loading} onclick={skipSync}>
            {m.initialsync_skip()}
          </Button>
        </div>
      </div>
    </div>
  </div>
</Modal>

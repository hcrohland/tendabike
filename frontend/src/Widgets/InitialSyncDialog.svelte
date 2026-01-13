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
  import { myfetch, handleError } from "../lib/store";
  import { user } from "../lib/user";

  let open = $state($user?.onboarding_status === "pending");

  let loading = $state(false);

  async function triggerSync() {
    loading = true;
    try {
      const updatedUser = await myfetch("/strava/onboarding/sync", "POST");
      $user = updatedUser;
      open = false;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }

  async function skipSync() {
    loading = true;
    try {
      const updatedUser = await myfetch("/strava/onboarding/postpone", "POST");
      user.set(updatedUser);
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
>
  <div class="text-center">
    <Heading
      tag="h3"
      class="mb-5 text-lg font-normal text-gray-500 dark:text-gray-400"
    >
      Welcome to TendaBike!
    </Heading>
    <P class="mb-4 text-left">
      Would you like to import your historic activities from Strava?
    </P>
    <P class="mb-6 text-left text-sm text-gray-500 dark:text-gray-400">
      This will sync all your past activities to help you track your bike
      maintenance.
    </P>
    <P class="mb-6 text-left text-sm text-gray-500 dark:text-gray-400">
      It will run in the background and may take a few minutes depending on how
      many activities you have.
    </P>
    <P class="mb-6 text-left text-sm text-gray-500 dark:text-gray-400">
      You can also skip this step. In that case you can pull the old activities
      later any time in the user menu under Sync &RightArrow; "Import Historic
      Activities"
    </P>
    <P class="mb-6 text-left text-sm text-gray-500 dark:text-gray-400">
      In any case the app will pull in new activities automatically. <br />
    </P>
    <div class="flex justify-center gap-4">
      <Button color="blue" disabled={loading} onclick={triggerSync}>
        {loading ? "Importing..." : "Import Activities"}
      </Button>
      <Button color="alternative" disabled={loading} onclick={skipSync}>
        Skip for Now
      </Button>
    </div>
  </div>
</Modal>

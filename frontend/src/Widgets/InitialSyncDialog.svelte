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
  import { m } from "../../paraglide/messages";

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
    <Heading tag="h3" class="mb-5 text-lg font-normal text-text-1">
      {m.initialsync_welcome()}
    </Heading>
    <P class="mb-4 text-left">
      {m.initialsync_question()}
    </P>
    <P class="mb-6 text-left text-sm text-text-1">
      {m.initialsync_desc1()}
    </P>
    <P class="mb-6 text-left text-sm text-text-1">
      {m.initialsync_desc2()}
    </P>
    <P class="mb-6 text-left text-sm text-text-1">
      {m.initialsync_desc3({
        sync: m.header_sync(),
        historic: m.header_import_historic(),
      })}
    </P>
    <P class="mb-6 text-left text-sm text-text-1">
      {m.initialsync_desc4()} <br />
    </P>
    <div class="flex justify-center gap-4">
      <Button color="blue" disabled={loading} onclick={triggerSync}>
        {loading ? m.action_importing() : m.header_import_activities()}
      </Button>
      <Button color="alternative" disabled={loading} onclick={skipSync}>
        {m.initialsync_skip()}
      </Button>
    </div>
  </div>
</Modal>

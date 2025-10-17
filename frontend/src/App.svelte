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

<script lang="ts" module>
  import Router from "svelte-spa-router";

  import About from "./About.svelte";
  import Activities from "./Activity/Activities.svelte";
  // import Admin from "./Admin/Admin.svelte";
  import Header from "./Header.svelte";
  // import Message from "./Message.svelte";
  import Gear from "./Part/Gear.svelte";
  import Plans from "./ServicePlan/Plans.svelte";
  import Spares from "./Spares/Spares.svelte";
  // import Statistics from "./Statistics.svelte";
  import ToyGroup from "./ToyGroup.svelte";
  import { initData } from "./lib/store";
  import { getTypes } from "./lib/types";

  const routes = {
    "/about": About,
    "/": ToyGroup,
    "/cat/": ToyGroup,
    "/part/:id": Gear,
    "/plans/": Plans,
    "/spares/": Spares,
    // "/admin": Admin,
    // "/stats": Statistics,
    "/activities/:part?/:start?": Activities,
  };

  await getTypes();
  let promise = initData();
</script>

<script lang="ts">
  import "./app.css";
  import { CardPlaceholder, ThemeProvider } from "flowbite-svelte";
  import Actions from "./Widgets/Actions.svelte";

  const theme = {
    tableBodyCell: "px-2 py-3",
    tableHeadCell: "px-2 py-3",
  };
</script>

<Header {promise} />

<!-- <Message /> -->
<ThemeProvider {theme}>
  <div class="m-8">
    {#await promise}
      <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
        {#each [0, 1, 2, 3] as i}
          <CardPlaceholder class="mb-4 p-4" />
        {/each}
      </div>
    {:then}
      <Router {routes} />
    {:catch error}
      <About />
    {/await}
  </div>
</ThemeProvider>
<Actions />

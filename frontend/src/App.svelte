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

<script lang="ts" context="module">
  import { Spinner, Container } from "@sveltestrap/sveltestrap";
  import Router from "svelte-spa-router";

  import ToyGroup from "./ToyGroup.svelte";
  import Header from "./Header.svelte";
  import Gear from "./Part/Part.svelte";
  import Spares from "./Spares/Spares.svelte";
  import About from "./About.svelte";
  import Message from "./Message.svelte";
  import Admin from "./Admin/Admin.svelte";
  import { getTypes, initData } from "./lib/store";
  import Statistics from "./Statistics.svelte";
  import Activities from "./Activity/Activities.svelte";

  const routes = {
    "/about": About,
    "/": ToyGroup,
    "/cat/": ToyGroup,
    "/part/:id": Gear,
    "/spares/": Spares,
    "/admin": Admin,
    "/stats": Statistics,
    "/activities/:part?/:start?": Activities,
  };

  await getTypes();
</script>

<Header />
<Message />
<Container class="mt-2">
  {#await initData()}
    <Spinner />
  {:then}
    <Router {routes} />
  {:catch error}
    <About />
  {/await}
</Container>

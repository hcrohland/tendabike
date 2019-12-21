<script>
  import {link, push, location} from 'svelte-spa-router'
  import {myfetch, types, category, parts} from "./store.js";
  import Await from './Await.svelte';

  let disabled = false;
  let promise = undefined;

  async function synchronize() {
    disabled = true;
    promise = myfetch('/strava/sync?batch=100')
      .then(data => parts.updateMap(data[1]))
      .then(() => disabled = false)
  }
</script>

<nav class="navbar navbar-expand-sm navbar-light bg-light mb-2 ">
    <a class="navbar-brand" href="#/">
      Tend a 

      {#if $category}
        <strong> {$category.name} </strong>
      {:else}
        Gear
      {/if}
    </a>
  <button class="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
    <span class="navbar-toggler-icon"></span>
  </button>

  <div class="collapse navbar-collapse" id="navbarSupportedContent">
    <ul class="navbar-nav ml-auto">
          <button on:click={synchronize} {disabled} class="dropdown-item">
            <Await {promise}>
              Sync 
            </Await>
          </button>
          <a href="/about" use:link class="dropdown-item">About</a>
    </ul>
  </div>
</nav>
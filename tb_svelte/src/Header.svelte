<script>
  import {link, push, location} from 'svelte-spa-router'
  import myfetch, {types, category, parts} from "./store.js";
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
  <!-- More links -->
  <div class="collapse navbar-collapse" id="navbarSupportedContent">
    <ul class="navbar-nav ml-auto">
      <!-- <li class="nav-item dropdown"> -->
        <!-- <a class="nav-link dropdown-toggle" href="#/" id="navbarDropdown" role="button" data-toggle="dropdown" aria-haspopup="true" aria-expanded="false">
          <span class="navbar-toggler-icon"></span>
        </a>
        <div class="dropdown-menu dropdown-menu-right" aria-labelledby="navbarDropdown"> -->
          <button on:click={synchronize} {disabled} class="dropdown-item">
            <Await {promise}>
              Sync 
            </Await>
          </button>
          <a href="/about" use:link class="dropdown-item">About</a>
          <!-- <a class="dropdown-item" href="#">Another action</a>
          <div class="dropdown-divider"></div>
          <a class="dropdown-item" href="#">Something else here</a> -->
        <!-- </div> -->
      <!-- </li> -->
    </ul>
    <!-- <form class="form-inline my-2 my-lg-0">
      <input class="form-control mr-sm-2" type="search" placeholder="Search" aria-label="Search">
      <button class="btn btn-outline-success my-2 my-sm-0" type="submit">Search</button>
    </form> -->
  </div>
</nav>
<script>
  import {link, push} from 'svelte-spa-router'
  import {types, gear, category} from "./store.js";
  $: categories = Object.keys($gear) || [];
</script>

<nav class="navbar navbar-expand-sm navbar-light bg-light mb-2 ">
  <button class="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
    <span class="navbar-toggler-icon"></span>
  </button>
  <div class="collapse navbar-collapse" id="navbarSupportedContent">
    <a class="navbar-brand" href="#/">Tend your </a>
  </div>

  <!-- Category selector -->
  <div class="btn-group mr-auto">
    <select bind:value={$category} class="border-0 custom-select custom-select-lg bg-secondary" id="exampleFormControlSelect1">
      <option hidden selected> what?</option>
      {#each categories as cat}
        <option value="{cat}"> {$types[cat].name}s </option>
      {/each}
    </select>
  </div>
  
  <!-- More links -->
  <div class="collapse navbar-collapse" id="navbarSupportedContent">
    <ul class="navbar-nav ml-auto">
      <li class="nav-item dropdown">
        <!-- <a class="nav-link dropdown-toggle" href="#/" id="navbarDropdown" role="button" data-toggle="dropdown" aria-haspopup="true" aria-expanded="false">
          <span class="navbar-toggler-icon"></span>
        </a>
        <div class="dropdown-menu dropdown-menu-right" aria-labelledby="navbarDropdown"> -->
          <a href="/about" use:link class="dropdown-item">About</a>
          <!-- <a class="dropdown-item" href="#">Another action</a>
          <div class="dropdown-divider"></div>
          <a class="dropdown-item" href="#">Something else here</a> -->
        <!-- </div> -->
      </li>
    </ul>
    <!-- <form class="form-inline my-2 my-lg-0">
      <input class="form-control mr-sm-2" type="search" placeholder="Search" aria-label="Search">
      <button class="btn btn-outline-success my-2 my-sm-0" type="submit">Search</button>
    </form> -->
  </div>
</nav>
<script>
export let promise;
</script>


{#await promise}
	<div class="d-flex justify-content-center">
    <div class="spinner-border" role="status">
      <span class="sr-only">Loading...</span>
    </div>
	</div>
{:then data}
    <slot {data}></slot>
{:catch error}
  {error}
{/await}
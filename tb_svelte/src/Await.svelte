<script>
export let promise;

function handleError(e) {
  alert(e)
  location.reload(); 
}
</script>


{#await promise}
	<div class="d-flex justify-content-center">
    <div class="spinner-border spinner-border-sm" role="status">
      <span class="sr-only">Loading...</span>
    </div>
	</div>
{:then data}
    <slot {data}></slot>
{:catch error}
  {handleError(error)}
{/await}
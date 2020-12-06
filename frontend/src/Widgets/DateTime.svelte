<script lang="ts">
  
  export let date = new Date;
	export let mindate = undefined;
	export let maxdate = undefined;

	const props = Object.assign({}, $$props);
	delete props.date;

  import Flatpickr from 'svelte-flatpickr'

	import 'flatpickr/dist/flatpickr.css'
	import 'flatpickr/dist/themes/light.css'

	const options = {
		year: 'numeric', month: 'short', day: 'numeric',
		hour: 'numeric', minute: 'numeric'
	}

	let flatpickrOptions = {
		time_24hr: true,
		enableTime: true,
		minuteIncrement: 15,
		formatDate: dateObj =>  dateObj.toLocaleString(navigator.language, options),
		minDate: mindate,
		maxDate: maxdate
	}
	export function roundTime(date: Date, minutes?: number) {
		if (!minutes) minutes = 15
		date = date ? new Date(date) : new Date()
    date.setMinutes(Math.floor(date.getMinutes()/15)*15)
    date.setSeconds(0)
    date.setMilliseconds(0)
    return date
	}	

	function handleChange(event) {
		const [ selectedDates ] = event.detail;
		date = selectedDates[0] as Date
	}
</script>

	<Flatpickr options={ flatpickrOptions } 
		value={roundTime(date)}
		on:change={handleChange}
		{...props}
	/> 
	{#if mindate}
		<button on:click={() => date = mindate}> 
			&#706;
		</button>
	{/if}
	{#if (!maxdate || (new Date(maxdate) >= new Date)) && (!mindate || (new Date(mindate) <= new Date)) }
		 <button on:click={() => date = new Date()}> 
			&#8226;
		</button>
	{/if}
	{#if maxdate}
		<button on:click={() => date = maxdate}> 
			&#707;
		</button>
	{/if}
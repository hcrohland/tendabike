<script lang="ts">
  
  export let date = new Date;
	export let mindate:any = undefined;
	export let maxdate:any = undefined;

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
    dateFormat: "j. M Y H:i",
		minDate: mindate,
		maxDate: maxdate
	}
	export function roundTime(date: Date, minutes?: number) {
		if (!minutes) minutes = 15
		date = date ? new Date(date) : new Date()
    date.setMinutes(Math.floor(date.getMinutes()/15)*15)
    date.setSeconds(0)
		date.setMilliseconds(0)
		if (date < mindate) date = mindate
		if (date > maxdate) date = maxdate
    return date
	}	

	function handleChange(event:any) {
		const [ selectedDates ] = event.detail;
		date = selectedDates[0] as Date
	}

	date = roundTime(date)
	
</script>

	<Flatpickr options={ flatpickrOptions } 
		value={date}
		on:change={handleChange}
		{...props}
	/> 
	<!-- hack to prevent spurious button clicks -->
	<button hidden on:click|preventDefault|stopPropagation={() => {}} tabindex="-1" /> 
	{#if mindate}
		<button on:click|preventDefault={() => date = mindate}> 
			&#706;
		</button>
	{/if}
	{#if (!maxdate || (new Date(maxdate) >= new Date)) && (!mindate || (new Date(mindate) <= new Date)) }
		 <button on:click|preventDefault={() => date = roundTime(new Date())}> 
			&#8226;
		</button>
	{/if}
	{#if maxdate}
		<button on:click|preventDefault={() => date = maxdate}> 
			&#707;
		</button>
	{/if}
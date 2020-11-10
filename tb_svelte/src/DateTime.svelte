<script>
  
  export let date = Date.now();
	export let mindate = undefined;

	const props = Object.assign({}, $$props);
	delete props.date;

	let mydate = roundTime(new Date(date))

	$: date = mydate.toISOString();
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
		formatDate: dateObj =>  dateObj.toLocaleString(undefined, options),
		minDate: mindate,
		// onChange: (selectedDates, dateStr, instance) => {
		// 	console.log('Options onChange handler', dateStr)
		// }
	}
	export function roundTime(date, minutes) {
    if (!minutes) minutes = 15
    date.setMinutes(Math.floor(date.getMinutes()/15)*15)
    date.setSeconds(0)
    date.setMilliseconds(0)
    return date
	}	
</script>

<form>
  <div>
    <Flatpickr options={ flatpickrOptions } 
      bind:value={mydate}
			{...props}
    /> 
  </div>
</form> 

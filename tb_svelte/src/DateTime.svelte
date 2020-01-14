<script>
  
  export let date = Date.now();

	const props = Object.assign({}, $$props);
	delete props.date;

	let mydate = new Date(date);

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
		// onChange: (selectedDates, dateStr, instance) => {
		// 	console.log('Options onChange handler', dateStr)
		// }
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

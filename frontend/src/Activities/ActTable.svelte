<script lang="ts">
	import {
		by,
		category,
		activities,
		parts,
		filterValues,
		fmtNumber,
		fmtSeconds,
	} from "../store";
	import SvelteTable from "../Widgets/SvelteTable.svelte";
	import { addToUsage, newUsage, type Activity, type Part } from "../types";
	import RangeSlider from "svelte-range-slider-pips";

	export let acts: Activity[] = filterValues($activities, (a) =>
		$category ? $category.acts.some((t) => t.id == a.what) : false,
	).sort(by("start"));

	const DAY = 24 * 3600000;

	let max = Math.floor(acts[0].start.getTime() / DAY);
	let min = Math.floor(acts[acts.length - 1].start.getTime() / DAY);
	let values = [min, max];

	$: rows = filterRows(acts, values);

	function filterRows(acts: Activity[], values: Number[]) {
		let start = new Date(values[0] * DAY);
		let end = new Date(values[1] * DAY);

		return acts.filter((a) => {
			let starttime = a.start.getTime();
			return start.getTime() <= starttime && end.getTime() + DAY > starttime;
		});
	}

	const formatter = (v) => new Date(v * DAY).toLocaleDateString();

	const gearname = (v) => {
		let x = $parts[v.gear];
		if (x) {
			return x.name;
		} else {
			return "-";
		}
	};

	let columns = [
		{
			key: "start",
			title: "Start",
			value: (v) => v.start,
			sortable: true,
			searchValue: (v) =>
				v.start.toLocaleDateString() + " " + v.start.toLocaleTimeString(),
			renderValue: (v) =>
				v.start
					? v.start.toLocaleDateString() + " " + v.start.toLocaleTimeString()
					: "",
		},
		{
			key: "name",
			title: "Name",
			value: (v) => v.name || "",
			searchValue: (v) => v.name,
			sortable: true,
		},
		{
			key: "gear",
			title: "Gear",
			value: gearname,
			sortable: true,
			filterValue: (v) => v.gear,
			filterOptions: (rows) => {
				let types = {};
				rows.forEach((row) => {
					let name = gearname(row);
					if (types[row.gear] === undefined)
						types[row.gear] = { name: name, value: row.gear };
				});
				// fix order
				types = Object.entries(types)
					.sort()
					.reduce((o, [k, v]) => ((o[k] = v), o), {});
				return Object.values(types);
			},
		},
		{
			key: "climb",
			title: "Climb",
			value: (v) => v.climb,
			renderValue: (v) => fmtNumber(v.climb),
			sortable: true,
			class: "text-end",
		},
		{
			key: "descend",
			title: "Descend",
			value: (v) => v.descend || v.climb,
			renderValue: (v) => fmtNumber(v.descend || v.climb),
			sortable: true,
			class: "text-end",
		},
		{
			key: "distance",
			title: "Distance",
			value: (v) => v.distance,
			renderValue: (v) => fmtNumber(v.distance),
			sortable: true,
			class: "text-end",
		},
		{
			key: "time",
			title: "Time",
			value: (v) => v.time,
			renderValue: (v) => fmtSeconds(v.time),
			sortable: true,
			class: "text-end",
		},
		{
			key: "duration",
			title: "Duration",
			value: (v) => v.duration,
			renderValue: (v) => fmtSeconds(v.duration),
			sortable: true,
			class: "text-end",
		},
	];

	const totalsFunc = (r: Activity[]) => {
		return r.reduce(
			(total, row) => {
				addToUsage(total, row);
				return total;
			},
			{
				name: "Totals:",
				...newUsage(),
			},
		);
	};
</script>

<RangeSlider
	{min}
	{max}
	range
	pushy
	pips
	first="label"
	last="label"
	float
	{formatter}
	bind:values
></RangeSlider>

<SvelteTable
	{columns}
	{rows}
	sortOrders={[-1, 1]}
	sortBy="start"
	{totalsFunc}
	classNameTable={["table"]}
	classNameThead={["table-secondary"]}
	classNameSelect={["custom-select"]}
/>

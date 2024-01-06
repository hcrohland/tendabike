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
	let selection={};

	let min, max, values = [];

	MiniMax(undefined)

	function MiniMax(gear) {
		let set = acts.filter((a) => !gear || a.gear == gear).map((a) => a.start.getTime()/DAY)
		min = Math.floor(set.reduce((res, start) => start < res ? start : res));
		max = Math.floor(set.reduce((res, start) => start > res ? start : res));
		values = [min, max]
	}

	$: MiniMax(selection.gear)

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

	const createFilterOptions = (acts) => {
				let types = {};
				acts.forEach((act) => {
					let name = gearname(act);
					if (types[act.gear] === undefined)
						types[act.gear] = { name: name, value: act.gear };
				});
				return Object.values(types).sort(by("value"));
	}

	let filterOptions = createFilterOptions(acts);

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
			filterOptions,
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
	bind:filterSelections="{selection}"
	classNameTable={["table"]}
	classNameThead={["table-secondary"]}
	classNameSelect={["custom-select"]}
/>

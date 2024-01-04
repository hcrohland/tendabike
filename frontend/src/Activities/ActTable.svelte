<script lang="ts">
  import {category, activities, parts, filterValues, fmtNumber, fmtSeconds} from '../store'
  import SvelteTable from "../Widgets/SvelteTable.svelte";
  import {addToUsage, newUsage, type Activity, type Part} from "../types"

  export let rows: Activity[] = filterValues($activities, (a) =>  $category.acts.some((t) => t.id == a.what))

  const gearname = v => {let x = $parts[v.gear]; if (x) {return x.name }else{return "-"}};

  let columns = [
    {
			key: "start",
			title: "Start",
			value: v => v.start,
			sortable: true,
      searchValue: v => v.start.toLocaleDateString() + ' ' + v.start.toLocaleTimeString(),
      renderValue: v => v.start ? v.start.toLocaleDateString() + ' ' + v.start.toLocaleTimeString() : '',
		},
		{
			key: "name",
			title: "Name",
			value: v => v.name || '',
			searchValue: v => v.name,
			sortable: true,
		},
    {
			key: "gear",
			title: "Gear",
			value: gearname,
			sortable: true,
			filterValue: v => v.gear,
      filterOptions: rows => {
				let types = {};
				rows.forEach(row => {
					let name = gearname(row);
					if (types[row.gear] === undefined)
						types[row.gear] = {name: name, value: row.gear};
				});
				// fix order
				types = Object.entries(types)
					.sort()
					.reduce((o, [k, v]) => ((o[k] = v), o), {});
				return Object.values(types);
			},},
    {
			key: "climb",
			title: "Climb",
			value: v => fmtNumber(v.climb),
			sortable: true,
			class: "text-end",
		},
    {
			key: "descend",
			title: "Descend",
			value: v => fmtNumber(v.descend || v.climb),
			sortable: true,
			class: "text-end",
		},
    {
			key: "distance",
			title: "Distance",
			value: v => fmtNumber(v.distance),
			sortable: true,
			class: "text-end",
		},
    {
			key: "time",
			title: "Time",
			value: v => fmtSeconds(v.time),
			sortable: true,
			class: "text-end",
		},
    {
			key: "duration",
			title: "Duration",
			value: v => fmtSeconds(v.duration),
			sortable: true,
			class: "text-end",
		}
  ];

	const totalsFunc = (r: Activity[]) => {
			return r.reduce((total, row) => {
				addToUsage(total, row);
				return total;
			}, 
			{
				name: "Totals:",
				... newUsage()
			}
		)
	}
</script>

<SvelteTable 
  {columns} 
  {rows} 
  sortOrders={[-1,1]} 
  sortBy=start 
	{totalsFunc}
  classNameTable={['table']}
  classNameThead={['table-secondary']}
  classNameSelect={['custom-select']} 
/>


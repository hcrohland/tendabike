<script lang="ts">
  import {category, activities, parts, filterValues, by} from '../store'
  import SvelteTable from "svelte-table";
  import type {Activity, Part} from "../types"

  export let rows: Activity[] = filterValues($activities, (a) =>  $category.acts.some((t) => t.id == a.what))

  const gearname = v => {let x = $parts[v.gear]; if (x) {return x.name }else{return "-"}};

  let columns = [
    {
			key: "start",
			title: "Start",
			value: v => v.start,
			sortable: true,
      searchValue: v => v.start.toLocaleDateString() + ' ' + v.start.toLocaleTimeString(),
      renderValue: v => v.start.toLocaleDateString() + ' ' + v.start.toLocaleTimeString(),
		},
		{
			key: "name",
			title: "Name",
			value: v => v.name,
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
			value: v => v.climb,
			sortable: true,
		},
    {
			key: "descend",
			title: "Descend",
			value: v => v.descend | v.climb,
			sortable: true,
		},
    {
			key: "distance",
			title: "Distance",
			value: v => v.distance,
			sortable: true,
		},
    {
			key: "time",
			title: "Time",
			value: v => v.time,
			sortable: true,
		}
  ];
</script>

<SvelteTable 
  {columns} 
  {rows} 
  sortOrder={-1} 
  sortBy=start 
  classNameTable={['table']}
  classNameThead={['table-secondary']}
  classNameSelect={['custom-select']} 
/>


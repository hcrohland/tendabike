import {writable, readable, derived} from "svelte/store";

function handleErrors(response) {
    if (response.ok) {
        return response;
    }

    if (response.status === 401) {
        window.location.replace("/login");
    }
    throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
}

export function myfetch (url) {
	return fetch(url)
		.then(handleErrors)
		.then(response => response.json())
};
export function filterValues(map, fn) { 
    return Object.values(map).filter(fn)
};

export function formatSeconds(sec_num) {
    var hours   = Math.floor(sec_num / 3600);
    var minutes = Math.floor((sec_num - (hours * 3600)) / 60);

    if (minutes < 10) {minutes = "0"+minutes;}
    return hours+':'+minutes;
}

export const category = writable(undefined);
export const parts = mapable("id");
export const types = mapable("id");

function mapField (field) {
    return (map, obj) => {
            map[obj[field]] = obj;
            return map;
        }
}

function mapable(fn) {
	const { subscribe, set, update } = writable({});

	return {
        subscribe,
        setMap: (arr) => {set(arr.reduce(mapField(fn),{}))},
		updateMap: (arr) => update(n => arr.reduce(mapField(fn), n)),
	};
}



export const icons = {
    "1": "flaticon-mountain-bike",
    "301": "flaticon-run",
    "302": "flaticon-snow",
    "303": "flaticon-ski",
}



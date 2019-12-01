import {writable} from "svelte/store";

function handleErrors(response) {
    if (response.ok) {
        return response;
    }

    if (response.status === 401) {
        window.location.replace("/login");
    }
    throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
}

export default function myfetch (url) {
	return fetch(url)
		.then(handleErrors)
		.then(response => response.json())
}

export const types = writable([]);
export const icons = {
    "1": "flaticon-mountain-bike",
    "301": "flaticon-run",
    "302": "flaticon-snow",
    "303": "flaticon-ski",
    "306": "flaticon-windsurf",
    "307": "flaticon-kitesurfing",
    "308": "flaticon-rowing"
}



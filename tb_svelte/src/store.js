import {writable} from "svelte/store";

function handleErrors(response) {
    if (!response.ok) {
        throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
    }
    return response;
	}

export default function myfetch (url) {
	return fetch(url)
		.then(handleErrors)
		.then(response => response.json())
}

export const types = writable([]);
export const category = writable(undefined);




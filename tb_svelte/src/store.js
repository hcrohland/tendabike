import {writable} from "svelte/store";
import _ from 'lodash';

function handleErrors(response) {
    if (!response.ok) {
        throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
    }
    return response;
	}

export default function fetch_store () { 
    return Promise.all([
		fetch('http://localhost:8000/types/part')
			.then(handleErrors)
			.then(response => response.json())
			.then(data => types.set(data)),
		fetch(`http://localhost:8000/part/mygear`)
			.then(handleErrors)
			.then(response => response.json())
            .then(data => gear.set(_.groupBy(data, "what")))
    ])
}

export const types = writable([]);
export const gear = writable([]);
export const category = writable(undefined);




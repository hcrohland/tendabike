import {writable, derived} from "svelte/store";
import _ from 'lodash';

export const types = writable([]);

export const category = writable("1");

async function fetchGear(set) {
    console.log("fetchgear");
    const res = await fetch(`http://localhost:8000/part/mygear`);
    const mygear = await res.json();
    let res2 = _.groupBy(mygear, "what");
    set(res2);
}

function createGear() {
	const { subscribe, set, update } = writable([],fetchGear);

	return {
		subscribe,
	};
}

export const gear = createGear();

export const parts = derived(
	[gear, category],
    ([$gear, $category], set ) => {
        set([])
        let res = $gear[$category];
        if ( res ) {
            set (res)
        } 
    },
    []
);

export const categories = derived(
	gear,
    ($gear, set ) => {
        let res = Object.keys($gear);
        if ( res ) {
            set (res)
        } 
    },
    []
);


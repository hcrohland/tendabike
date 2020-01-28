import {writable, readable, derived} from "svelte/store";

function checkStatus(response) {
    if (response.ok) {
        return response;
    }

    if (response.status === 401) {
        window.location.replace("/login");
    }
    throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
}

export function myfetch (url, method, data) {
    let option
    if (method) {
        option = {
            method: method, // *GET, POST, PUT, DELETE, etc.
            credentials: 'include', // include, *same-origin, omit
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(data) // body data type must match "Content-Type" header
        };
    }
	return fetch(url, option)
		.then(checkStatus)
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

export function by(field) {
    return (a,b) => a[field] < b[field]
}

export function handleError(e) {
    alert(e)
    location.reload(); 
}

function mapObject (fn) {
    return (map, obj) => {
            map[fn(obj)] = obj;
            return map;
        }
}

function mapable(fn) {
    const { subscribe, set, update } = writable({});

	return {
        subscribe,
        setMap: (arr) => {set(arr.reduce(mapObject(fn),{}))},
		updateMap: (arr) => update(n => arr.reduce(mapObject(fn), n)),
	};  
}

export const icons = {
    "1": "flaticon-mountain-bike",
    "301": "flaticon-run",
    "302": "flaticon-snow",
    "303": "flaticon-ski",
}

export function initData () {
    return Promise.all([
        myfetch('/types/part')
            .then(types.setMap),
        myfetch('/part/all')
            .then(setPartAttach),
        myfetch('/user')
            .then(user.set)
])
}

export function isAttached (att, time) {
    return time >= new Date(att.attached) && (!att.detached || time < new Date(att.detached))
}

function setPartAttach(data) {
    parts.setMap(data.parts)
    attachments.setMap(data.attachments.map(a => {a.a.name = a.name; a.a.what=a.what; return a.a}))
}

export function updatePartAttach(data) {
    parts.updateMap(data.parts)
    attachments.updateMap(data.attachments.map(a => {a.a.name = a.name; a.a.what=a.what; return a.a}))
}

export const category = writable(undefined);
export const parts = mapable((o) => o["id"]);
export const types = mapable((o) => o["id"]);
export const user = writable();
export const attachments = mapable((o) => o["part_id"] + o["attached"])
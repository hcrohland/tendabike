import {writable, readable, derived} from "svelte/store";
import type {Part, Attachment, Type} from './types'; 

export function checkStatus<T>(response) {
    if (response.ok) {
        return response.json()
    }

    return response.text()
        .then((text) => {
            return Promise.reject(response.statusText + ': ' + text + ' accessing ' + response.url);
        })
}

export function myfetch (url, method?, data?) {
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
};

export function filterValues<T>(map, fn) { 
    return Object.values(map).filter(fn) as T[]
};

export function formatSeconds(sec_num) {
    var hours   = Math.floor(sec_num / 3600);
    var minutes: number | string = Math.floor((sec_num - (hours * 3600)) / 60);

    if (minutes < 10) {minutes = "0"+minutes;}
    return hours+':'+minutes;
}

export function by<T>(field: keyof T) {
    return (a: T,b: T) => a[field] < b[field]? 1 : -1
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

function mapable<K,V>(fn: (v: V) => K) {
    const { subscribe, set, update } = writable<V[]>([]);

	return {
        subscribe,
        setMap: (arr: V[]) => {set(arr.reduce(mapObject(fn),{}))},
		updateMap: (arr: V[]) => update(n => arr.reduce(mapObject(fn), n)),
	};  
}

export const icons = {
    "1": "flaticon-mountain-bike",
    "301": "flaticon-run",
    "302": "flaticon-snow",
    "303": "flaticon-ski",
}

export async function initData () {
    var u = await myfetch('/user')
    if (u) {
        user.set(u)
    } else {
        return
    }
    return Promise.all([
        myfetch('/types/part')
            .then(types.setMap),
        myfetch('/part/all')
            .then(setPartAttach),
])
}

export function isAttached (att, time) {
    return time >= new Date(att.attached) && (!att.detached || time < new Date(att.detached))
}

export function setPartAttach(data) {
    parts.setMap(data.parts)
    attachments.setMap(data.attachments)
}

export function updatePartAttach(data) {
    parts.updateMap(data.parts)
    attachments.updateMap(data.attachments)
}

export const category = writable(undefined);
export const parts = mapable((o: Part) => o["id"]);
export const types = mapable((o: Type) => o["id"]);
export const user = writable(undefined);
export const attachments = mapable((o:Attachment) => o["part_id"].toString() + o["attached"].toString())
export const state = writable({ show_all_spares: false});
import { writable, get } from "svelte/store";
import type { Part, Attachment, Type, Activity, ActType } from './types';

export const maxDate = new Date("2999-12-31");
export function checkStatus<T>(response) {
    if (response.ok) {
        return response.json()
    }

    if (response.status === 401) {
        user.set(undefined);
        window.location.href = '/#/about';
    }

    return response.text()
        .then((text) => {
            message.set({ active: true, status: response.statusText, message: text })
            return Promise.reject(text)
        })
}

export function fmtDate(date) {
    return new Date(date).toLocaleDateString(navigator.language)
}

export function fmtNumber(number: number) {
    return number.toLocaleString(navigator.language)
}

export function myfetch(url, method?, data?) {
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

export function filterValues<T>(map: { [key: string]: T }, fn: (t: T) => boolean) {
    return Object.values(map).filter(fn)
};

export function fmtSeconds(sec_num) {
    var hours = Math.floor(sec_num / 3600);
    var minutes: number | string = Math.floor((sec_num - (hours * 3600)) / 60);

    if (minutes < 10) { minutes = "0" + minutes; }
    return hours + ':' + minutes;
}

export function by<T>(field: keyof T, asc?: boolean) {
    return (a: T, b: T) => (a[field] < b[field] ? 1 : -1) * (asc ? -1 : 1)
}

export function handleError(e) {
    message.update(m => {
        m.message = e;
        m.active = true;
        return m
    })
}

function mapObject<T>(fn, del?): (a, b) => { [key: string]: T } {
    return (map, obj) => {
        if (del && del(obj))
            delete map[fn(obj)]
        else
            map[fn(obj)] = obj;
        return map;
    }
}

function mapable<K, V>(field: string, mapfn?: (v: any) => V, delfn?: (v: V) => boolean) {
    if (!mapfn) mapfn = (v) => v;
    const { subscribe, set, update } = writable<{ [key: string]: V }>({});
    const fn = (v) => v[field];

    return {
        subscribe,
        setMap: (arr: V[]) => { set(arr.map(mapfn).reduce(mapObject(fn, delfn), {})) },
        updateMap: (arr: V[]) => update(n => arr.map(mapfn).reduce(mapObject(fn, delfn), n)),
    };
}

export const icons = {
    "1": "flaticon-mountain-bike",
    "301": "flaticon-run",
    "302": "flaticon-snow",
    "303": "flaticon-ski",
}

export async function initData() {
    let u = await myfetch('/user')
    if (u) {
        user.set(u)
    } else {
        return
    }
    return Promise.all([
        myfetch('/types/part')
            .then((data) => data.map(prepTypes).reduce(mapObject((t) => t.id), {})),
        myfetch('/types/activity'),
        myfetch('/user/summary')
            .then(setSummary),
    ])
        .then((t) => {
            types = t[1].reduce(
                (acc, a: ActType) => {
                    (acc[a.gear_type] as Type).acts.push(a);
                    return acc
                },
                t[0]
            );
        })
        .then(() => category.set(types[1]))
}

export function isAttached(att: Attachment, time?) {
    if (!time) time = new Date()
    return att.attached <= time && time < att.detached
}

export function setSummary(data) {
    parts.setMap(data.parts);
    attachments.setMap(data.attachments);
    activities.setMap(data.activities);
}

export function updateSummary(data) {
    parts.updateMap(data.parts);
    attachments.updateMap(data.attachments);
    activities.updateMap(data.activities);
}

export let types: { [key: number]: Type };
function prepTypes(t: Type) {
    t.prefix = t.name.split(' ').reverse()[1] || '';  // The first word iff there were two (hack!)
    t.acts = [];
    return t
}

export const category = writable(undefined);
export const user = writable(undefined);

export const parts = mapable("id", prepParts);
export const activities = mapable("id", prepActs)
export const attachments = mapable("idx", prepAtts, delAtt)
function prepParts(a: Part) { a.purchase = new Date(a.purchase); return a }
function prepActs(a: Activity) { a.start = new Date(a.start); return a }
export function attTime(att: Attachment) {
    let res = fmtDate(att.attached);
    if (att.detached < maxDate)
        res = res + " - " + fmtDate(att.detached)
    return res
}
function prepAtts(a: Attachment) {
    a.attached = new Date(a.attached);
    a.detached = new Date(a.detached);
    a.idx = a.part_id + "/" + a.attached.getTime()
    return a
}
function delAtt(o) { return o.attached.getTime() == o.detached.getTime() }

export const state = writable({ show_all_spares: false });
export const message = writable({ active: false, message: "No message", status: "" })
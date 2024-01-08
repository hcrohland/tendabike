import { writable } from "svelte/store";
import { mapable, mapObject} from "./mapable";
import { Activity, Part, Attachment, type Type, type ActType, type User } from './types';

export { filterValues, by } from './mapable';

export function fmtDate(date: Date | string | number) {
    return new Date(date).toLocaleDateString(navigator.language)
}

export function fmtSeconds(sec_num: number | undefined) {
    sec_num = sec_num || 0;
    var hours = Math.floor((sec_num) / 3600);
    var minutes: number | string = Math.floor((sec_num - (hours * 3600)) / 60);

    if (minutes < 10) { minutes = "0" + minutes; }
    return hours + ':' + minutes;
}

export function fmtNumber(number: number | undefined) {
    return (number || 0).toLocaleString(navigator.language)
}

export function myfetch(url: string, method?: any, data?: any) {
    let option: RequestInit | undefined;
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

export function checkStatus<T>(response: Response) {
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

export function handleError(e: Error) {
    message.update(m => {
        m.message = e.message;
        m.active = true;
        return m
    })
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
            .then((types) => types.map(prepTypes).reduce(mapObject('id'), {})), // data[0]
        myfetch('/types/activity'), // data[1]
        myfetch('/user/summary')
            .then(setSummary),
    ])
        .then((data: { 0: Type[], 1: ActType[] }) => {
            types = data[1].reduce(
                (acc, a) => {
                    (acc[a.gear_type]).acts.push(a);
                    return acc
                },
                data[0]
            );
        })
        .then(() => category.set(types[1]))

}

type Summary = { parts: Part[], attachments: Attachment[], activities: Activity[] }

export function setSummary(data: Summary) {
    parts.setMap(data.parts);
    attachments.setMap(data.attachments);
    activities.setMap(data.activities);
}

export function updateSummary(data: Summary) {
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

export const category = writable<Type | undefined>(undefined);
export const user = writable<User | undefined>(undefined);

export const parts = mapable("id", prepParts);
function prepParts(a: Part) { return new Part(a) }

export const activities = mapable("id", prepActs)
function prepActs(a: Activity) { return new Activity(a) }

export const attachments = mapable("idx", prepAtts, delAtt)
function prepAtts(a: Attachment) { return new Attachment(a) }
function delAtt(a: Attachment) { return a.attached.getTime() == a.detached.getTime() }

export const state = writable({ show_all_spares: false });
export const message = writable({ active: false, message: "No message", status: "" })
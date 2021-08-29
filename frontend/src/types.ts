export type Usage = {
  count?: number;
  climb?: number; 
  descend?: number; 
  distance?: number; 
  time?:  number;  
  duration?: number; 
}

export function newUsage(): Usage {
  return {
    count: 0,
    climb: 0,
    descend: 0,
    distance: 0,
    time: 0,
    duration: 0
  }
}

export function addToUsage (u: Usage, a: Usage) {
  u.count += a.count || 0;
  u.climb += a.climb || 0;
  u.descend += a.descend || u.climb;
  u.distance += a.distance || 0;
  u.time += a.time || a.duration || 0;
  u.duration += a.duration || u.time;
}

export type Part = Usage & {
    id?: number;
    owner: number;
    what: number; 
    name: string; 
    vendor: string; 
    model: string; 
    purchase: Date;
    last_used: Date;
    disposed_at?: Date;
}

export type Attachment = Usage & {
    part_id: number;
    attached: Date;
    gear: number;
    hook: number;
    detached: Date;
    what: number;
    name: string;
    part: Part;
    idx: string;
  }


export type Type = {
    id: number;
    name: string;
    main: number;
    hooks: Array<number>;
    order: number;
    group?: string;
    prefix: string;
    acts: ActType[];
}

export type User = {
  id: number,
  firstname: string,
  name: string,
  is_admin: boolean
}

export type Activity = Usage & {
  id: number,
  /// The athlete
  user_id: number,
  /// The activity type
  what: number,
  /// This name of the activity.
  name: string,
  /// Start time
  start: Date,
  /// Which gear did she use?
  gear?: number,
}

export type ActType = {
  id: number,
  name: string,
  gear_type: number
}

export type AttEvent = {
  part_id: number;
  time: Date;
  gear: number;
  hook: number;
}
  
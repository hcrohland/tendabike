export type Usage = {
  count?: number;
  climb?: number; 
  descend?: number; 
  distance?: number; 
  time?:  number;  
  duration?: number; 
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
    gear?: number;
    hook?: number;
    detached?: Date;
    what?: number;
    name?: string;
  }


export type Type = {
    id: number;
    name: string;
    main: number;
    hooks: Array<number>;
    order: number;
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
  
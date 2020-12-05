export type Part = {
    id?: number;
    owner: number;
    what: number; 
    count: number;
    climb: number; 
    descend: number; 
    distance: number; 
    time:  number;
    name: string; 
    vendor: string; 
    model: string; 
    purchase: Date;
    last_used: Date;
    disposed_at?: Date;
}

export type Attachment = {
    part_id: number;
    attached: Date;
    gear?: number;
    hook?: number;
    detached?: Date;
    what?: number;
    name?: string;
    count: number;
    climb: number; 
    descend: number; 
    distance: number; 
    time:  number;
  }

export type Type = {
    id: number;
    name: string;
    main: number;
    hooks: Array<number>;
    order: number;
  }
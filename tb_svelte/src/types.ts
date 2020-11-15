export interface Part {
    id: number | undefined;
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
    purchase: number;
}

export interface Attach {
    part_id: number;
    attached: number;
    gear: number | undefined,
    hook: number | undefined,
  }
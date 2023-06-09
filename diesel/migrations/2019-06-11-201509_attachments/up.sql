
CREATE TABLE attachments (
    part_id integer REFERENCES parts(id),
    attached timestamp with time zone,
    gear integer NOT NULL REFERENCES parts(id),
    hook integer NOT NULL REFERENCES part_types(id),
    detached timestamp with time zone,
    CONSTRAINT attachments2_pkey PRIMARY KEY (part_id, attached)
);

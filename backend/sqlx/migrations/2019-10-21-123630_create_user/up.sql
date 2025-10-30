-- Your SQL goes here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    firstname text NOT NULL,
    is_admin BOOLEAN NOT NULL
);
-- Your SQL goes here

CREATE TABLE roles (
    id SERIAL PRIMARY KEY,
    role_id UUID NOT NULL,
    role_code VARCHAR NOT NULL,
    role_name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL, -- enabled
    permissions TEXT NOT NULL, -- JSON
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    extra TEXT NULL -- JSON
);

CREATE UNIQUE INDEX roles_role_id_key ON roles
(role_id);

CREATE UNIQUE INDEX roles_role_code_key ON roles
(role_code);

CREATE INDEX roles_is_enabled_idx ON roles
(is_enabled);

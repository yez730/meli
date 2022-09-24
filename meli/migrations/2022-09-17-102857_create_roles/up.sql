-- Your SQL goes here

CREATE TABLE roles (
    id BIGSERIAL PRIMARY KEY,
    role_id UUID NOT NULL,
    role_code VARCHAR NOT NULL,
    role_name VARCHAR NOT NULL,
    permissions TEXT NOT NULL,
    description TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX roles_role_id_key ON roles
(role_id);

CREATE UNIQUE INDEX roles_role_code_key ON roles
(role_code);

CREATE INDEX roles_enabled_idx ON roles
(enabled);

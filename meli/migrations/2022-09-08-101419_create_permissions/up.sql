-- Your SQL goes here

CREATE TABLE permissions (
    id BIGSERIAL PRIMARY KEY,
    permission_id UUID NOT NULL,
    permission_code VARCHAR NOT NULL,
    permission_name VARCHAR NOT NULL,
    description TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX permissions_permission_id_key ON permissions
(permission_id);

CREATE UNIQUE INDEX permissions_permission_code_key ON permissions
(permission_code);

CREATE INDEX permissions_enabled_idx ON permissions
(enabled);

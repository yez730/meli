-- Your SQL goes here

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    username VARCHAR NOT NULL,
    description TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL, -- enabled
    roles TEXT NOT NULL, -- JSON
    permissions TEXT NOT NULL, -- JSON
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    extra TEXT NULL -- JSON
);

CREATE UNIQUE INDEX users_user_id_key ON users
(user_id);

CREATE INDEX users_is_enabled_idx ON users
(is_enabled);

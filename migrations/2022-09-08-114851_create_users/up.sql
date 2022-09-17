-- Your SQL goes here

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    username VARCHAR NOT NULL,
    description TEXT NOT NULL,
    permissions TEXT NOT NULL,
    roles TEXT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX users_user_id_key ON users
(user_id);

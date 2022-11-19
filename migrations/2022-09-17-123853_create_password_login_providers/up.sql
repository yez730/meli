-- Your SQL goes here

CREATE TABLE password_login_providers (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    password_hash TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX password_login_providers_user_id_key ON password_login_providers
(user_id);

CREATE INDEX password_login_providers_enabled_idx ON password_login_providers
(enabled);

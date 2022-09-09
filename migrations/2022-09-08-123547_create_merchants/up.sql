-- Your SQL goes here

CREATE TABLE merchants (
    id BIGSERIAL PRIMARY KEY,
    merchant_id UUID NOT NULL,
    merchant_name VARCHAR NOT NULL,
    is_enabled BOOLEAN NOT NULL, -- enabled
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    extra TEXT NULL -- JSON
);

CREATE UNIQUE INDEX merchants_merchant_id_key ON merchants
(merchant_id);

CREATE INDEX merchants_is_enabled_idx ON merchants
(is_enabled);

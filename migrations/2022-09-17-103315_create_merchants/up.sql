-- Your SQL goes here

CREATE TABLE merchants (
    id BIGSERIAL PRIMARY KEY,
    merchant_id UUID NOT NULL,
    merchant_name VARCHAR NOT NULL,
    company_name VARCHAR NULL,
    credential_no VARCHAR NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX merchants_merchant_id_key ON merchants
(merchant_id);

CREATE INDEX merchants_merchant_name_idx ON merchants
(merchant_name);

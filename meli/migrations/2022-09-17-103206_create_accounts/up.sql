-- Your SQL goes here

CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    account_id UUID NOT NULL,
    merchant_id UUID NOT NULL,
    cellphone VARCHAR NOT NULL,
    email VARCHAR NULL,
    real_name VARCHAR NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX accounts_user_id_key ON accounts
(user_id);

CREATE UNIQUE INDEX accounts_account_id_key ON accounts
(account_id);

CREATE INDEX accounts_merchant_id_idx ON accounts
(merchant_id);

CREATE INDEX accounts_cellphone_idx ON accounts
(cellphone);

CREATE INDEX accounts_email_idx ON accounts
(email);

CREATE INDEX accounts_enabled_idx ON accounts
(enabled);

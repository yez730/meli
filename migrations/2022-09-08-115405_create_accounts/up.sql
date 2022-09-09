-- Your SQL goes here

CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    account_id UUID NOT NULL,
    merchant_id UUID NOT NULL, -- 某个商户
    cellphone VARCHAR NOT NULL,
    account_name VARCHAR NOT NULL,
    account_state VARCHAR NOT NULL,
    account_photo_url TEXT NULL,
    credential_type VARCHAR NULL,
    credential_no VARCHAR NULL,
    real_name VARCHAR NULL,
    emp_no VARCHAR NULL,
    email VARCHAR NULL,
    is_enabled BOOLEAN NOT NULL, -- enabled
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    extra TEXT NULL -- JSON
);

CREATE INDEX accounts_user_id_idx ON accounts
(user_id);

CREATE INDEX accounts_account_id_idx ON accounts
(account_id);

CREATE INDEX accounts_merchant_id_idx ON accounts
(merchant_id);

CREATE INDEX accounts_cellphone_idx ON accounts
(cellphone);

CREATE INDEX accounts_is_enabled_idx ON accounts
(is_enabled);

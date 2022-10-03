-- Your SQL goes here

CREATE TABLE merchant_members (
    id BIGSERIAL PRIMARY KEY,
    merchant_id UUID NOT NULL,
    member_id UUID NOT NULL,
    balance MONEY NOT NULL,
    enabled BOOLEAN NOT NULL, 
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE INDEX merchant_members_merchant_id_idx ON merchant_members
(merchant_id);

CREATE INDEX merchant_members_member_id_idx ON merchant_members
(member_id);

CREATE INDEX merchant_members_enabled_idx ON merchant_members
(enabled);

-- Your SQL goes here

CREATE TABLE recharge_records (
    id BIGSERIAL PRIMARY KEY,
    recharge_record_id UUID NOT NULL,
    member_id UUID NOT NULL,
    amount MONEY NOT NULL,
    enabled BOOLEAN NOT NULL, 
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX recharge_records_recharge_record_id_key ON recharge_records
(recharge_record_id);

CREATE INDEX recharge_records_enabled_idx ON recharge_records
(enabled);

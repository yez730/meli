-- Your SQL goes here

CREATE TABLE service_types (
    id BIGSERIAL PRIMARY KEY,
    service_type_id UUID NOT NULL,
    merchant_id UUID NOT NULL,
    name VARCHAR NOT NULL,
    estimated_duration INT NOT NULL, -- 分钟
    normal_prize NUMERIC NOT NULL,
    member_prize NUMERIC NOT NULL,
    enabled BOOLEAN NOT NULL, 
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX service_types_recharge_record_id_key ON service_types
(service_type_id);

CREATE INDEX service_types_merchant_id_idx ON service_types
(merchant_id);

CREATE INDEX service_types_enabled_idx ON service_types
(enabled);

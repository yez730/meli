-- Your SQL goes here

CREATE TABLE barbers (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    barber_id UUID NOT NULL,
    merchant_id UUID NOT NULL,
    cellphone VARCHAR NOT NULL,
    email VARCHAR NULL,
    real_name VARCHAR NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX barbers_user_id_key ON barbers
(user_id);

CREATE UNIQUE INDEX barbers_barber_id_key ON barbers
(barber_id);

CREATE INDEX barbers_merchant_id_idx ON barbers
(merchant_id);

CREATE INDEX barbers_cellphone_idx ON barbers
(cellphone);

CREATE INDEX barbers_email_idx ON barbers
(email);

CREATE INDEX barbers_enabled_idx ON barbers
(enabled);

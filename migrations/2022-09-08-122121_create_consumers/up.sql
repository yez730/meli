-- Your SQL goes here

CREATE TABLE consumers (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    consumer_id UUID NOT NULL,
    cellphone VARCHAR NOT NULL,
    email VARCHAR NULL,
    credential_no VARCHAR NULL,
    real_name VARCHAR NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX consumers_user_id_key ON consumers
(user_id);

CREATE UNIQUE INDEX consumers_consumer_id_key ON consumers
(consumer_id);

CREATE INDEX consumers_cellphone_idx ON consumers
(cellphone);

CREATE INDEX consumers_email_idx ON consumers
(email);

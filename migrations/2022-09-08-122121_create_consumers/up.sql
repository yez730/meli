-- Your SQL goes here

CREATE TABLE consumers (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    consumer_id UUID NOT NULL,
    cellphone VARCHAR NOT NULL,
    real_name VARCHAR NULL,
    credential_type VARCHAR NULL,
    credential_no VARCHAR NULL,
    selfie_photo_url TEXT NULL,
    is_verified BOOLEAN NOT NULL,
    two_factor_verify BOOLEAN NOT NULL,
    is_enabled BOOLEAN NOT NULL, -- enabled
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    extra TEXT NULL -- JSON
);

CREATE UNIQUE INDEX consumers_user_id_key ON consumers
(user_id);

CREATE UNIQUE INDEX consumers_consumer_id_key ON consumers
(consumer_id);

CREATE INDEX consumers_cellphone_idx ON consumers
(cellphone);

CREATE INDEX consumers_is_enabled_idx ON consumers
(is_enabled);

CREATE INDEX consumers_consumer_id_is_enabled_idx ON consumers
(consumer_id,is_enabled);

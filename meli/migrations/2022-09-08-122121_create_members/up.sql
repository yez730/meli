-- Your SQL goes here

CREATE TABLE members (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    member_id UUID NOT NULL,
    cellphone VARCHAR NOT NULL,
    real_name VARCHAR NULL,
    gender VARCHAR NULL,
    birth_day DATE NULL,
    enabled BOOLEAN NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX members_user_id_key ON members
(user_id);

CREATE UNIQUE INDEX members_member_id_key ON members
(member_id);

CREATE INDEX members_cellphone_idx ON members
(cellphone);

CREATE INDEX members_enabled_idx ON members
(enabled);

-- Your SQL goes here

CREATE TABLE login_infos (
    id BIGSERIAL PRIMARY KEY,
    login_info_id UUID NOT NULL,
    login_info_barber VARCHAR NOT NULL,
    login_info_type VARCHAR NOT NULL,
    user_id UUID NOT NULL,
    enabled BOOLEAN NOT NULL ,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX login_infos_login_info_id_key ON login_infos
(login_info_id);

CREATE INDEX login_infos_login_info_barber_idx ON login_infos
(login_info_barber);

CREATE INDEX login_infos_user_id_idx ON login_infos
(user_id);

CREATE INDEX login_infos_enabled_idx ON login_infos
(enabled);

CREATE INDEX login_infos_login_info_barber_login_info_type_idx ON login_infos
(login_info_barber,login_info_type);

CREATE INDEX login_infos_login_info_barber_login_info_type_enabled_idx ON login_infos
(login_info_barber,login_info_type,enabled);

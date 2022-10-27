-- Your SQL goes here

CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    user_id UUID NOT NULL,
    init_time TIMESTAMPTZ NOT NULL,
    expiry_time TIMESTAMPTZ NOT NULL,

    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NOT NULL
);

CREATE UNIQUE INDEX sessions_session_id_key ON sessions
(session_id);

CREATE INDEX sessions_user_id_idx ON sessions
(user_id);

-- Your SQL goes here

CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    data TEXT NOT NULL, -- keep track of browser auth state
    expiry_time TIMESTAMPTZ NOT NULL,
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX sessions_session_id_key ON sessions
(session_id);

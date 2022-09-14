-- Your SQL goes here

CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    expiry_time TIMESTAMPTZ NOT NULL, -- TODO  null
    extra TEXT NOT NULL
);

CREATE UNIQUE INDEX sessions_session_id_key ON sessions
(session_id);

-- CREATE TABLE sessions (
--     id BIGSERIAL PRIMARY KEY,
--     session_id UUID NOT NULL,
--     user_id UUID NOT NULL,
--     username VARCHAR NOT NULL,
--     client_id UUID NOT NULL,
--     client_ip VARCHAR NOT NULL,
--     client_type VARCHAR NOT NULL,
--     source_request_id UUID NOT NULL,
--     init_time TIMESTAMPTZ NOT NULL,
--     expiry_time TIMESTAMPTZ NOT NULL,
--     create_time TIMESTAMPTZ NOT NULL,
--     update_time TIMESTAMPTZ NOT NULL,
--     extra TEXT NULL -- JSON
-- );

-- CREATE UNIQUE INDEX sessions_session_id_key ON sessions
-- (session_id);

-- CREATE INDEX sessions_user_id_idx ON sessions
-- (user_id);

-- CREATE INDEX sessions_client_id_idx ON sessions
-- (client_id);

-- CREATE INDEX sessions_client_ip_idx ON sessions
-- (client_ip);

-- CREATE INDEX sessions_source_request_id_idx ON sessions
-- (source_request_id);

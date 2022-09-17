-- This file should undo anything in `up.sql`

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

DO
LANGUAGE plpgsql $$
DECLARE
  session_uuid UUID := uuid_generate_v4();
BEGIN
  INSERT INTO sessions(session_id,data,expiry_time,create_time,update_time)  VALUES(session_uuid,'',now(),now(),now());
END;
$$;

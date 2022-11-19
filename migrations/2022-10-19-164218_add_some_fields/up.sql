-- Your SQL goes here

ALTER TABLE login_infos RENAME login_info_barber TO login_info_account;
ALTER TABLE members ADD remark TEXT NULL;
ALTER TABLE merchants ADD address TEXT NULL;
ALTER TABLE merchants ADD remark TEXT NULL;

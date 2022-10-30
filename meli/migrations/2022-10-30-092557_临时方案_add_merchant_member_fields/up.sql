-- Your SQL goes here

-- member 信息暂时存储在特定商户下
ALTER TABLE merchant_members ADD cellphone VARCHAR NULL;
ALTER TABLE merchant_members ADD real_name VARCHAR NULL;
ALTER TABLE merchant_members ADD gender VARCHAR NULL;
ALTER TABLE merchant_members ADD birth_day DATE NULL;
ALTER TABLE merchant_members ADD remark TEXT NULL;

UPDATE merchant_members
SET cellphone = members.cellphone, real_name = members.real_name, gender = members.gender, birth_day = members.birth_day, remark = members.remark
FROM members
WHERE merchant_members.member_id=members.member_id;

ALTER TABLE merchant_members ALTER cellphone SET NOT NULL;
ALTER TABLE merchant_members ALTER real_name SET NOT NULL;

DROP TABLE members;

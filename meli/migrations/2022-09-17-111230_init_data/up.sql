-- Your SQL goes here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'AlphaAdministrator','超级管理员','最高权限管理员',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'MerchantAdministrator','商户管理员','商户管理员',true,now(),now(),null);

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Barber','商户普通权限','商户普通权限',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Member','顾客普通权限','顾客普通权限',true,now(),now(),null);

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Shouye','首页','首页',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'QiantaiYingye','前台营业','前台营业',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'KehuGuanli','客户管理','客户管理',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'YewuTongji','业务统计','业务统计',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'HoutaiGuanli','后台管理','后台管理',true,now(),now(),null);

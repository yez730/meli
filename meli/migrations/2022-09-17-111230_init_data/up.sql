-- Your SQL goes here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'AlphaAdministrator','超级管理员','最高权限管理员',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'MerchantAdministrator','商户所有者','商户所有者',true,now(),now(),null);

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Barber_Base','商户用户普通权限','普通理发师可以使用的权限',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Member_Base','顾客普通权限','顾客普通权限',true,now(),now(),null);

INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Canlendar','首页','首页',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Member','客户管理','客户管理',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Barber','员工管理','员工管理',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'Statistic','业务统计','业务统计',true,now(),now(),null);
INSERT INTO permissions (permission_id,permission_code,permission_name,description,enabled,create_time,update_time,data) VALUES(uuid_generate_v4(),'ServiceType','服务类型管理','服务类型管理',true,now(),now(),null);

// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Int8,
        user_id -> Uuid,
        account_id -> Uuid,
        merchant_id -> Uuid,
        cellphone -> Varchar,
        email -> Nullable<Varchar>,
        real_name -> Nullable<Varchar>,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    consumers (id) {
        id -> Int8,
        user_id -> Uuid,
        consumer_id -> Uuid,
        cellphone -> Varchar,
        real_name -> Nullable<Varchar>,
        gender -> Nullable<Varchar>,
        birth_day -> Nullable<Date>,
        balance -> Nullable<Money>,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    login_infos (id) {
        id -> Int8,
        login_info_id -> Uuid,
        login_info_account -> Varchar,
        login_info_type -> Varchar,
        user_id -> Uuid,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
    }
}

diesel::table! {
    merchants (id) {
        id -> Int8,
        merchant_id -> Uuid,
        merchant_name -> Varchar,
        company_name -> Nullable<Varchar>,
        credential_no -> Nullable<Varchar>,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    password_login_providers (id) {
        id -> Int8,
        user_id -> Uuid,
        password_hash -> Text,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    permissions (id) {
        id -> Int8,
        permission_id -> Uuid,
        permission_code -> Varchar,
        permission_name -> Varchar,
        description -> Text,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    roles (id) {
        id -> Int8,
        role_id -> Uuid,
        role_code -> Varchar,
        role_name -> Varchar,
        permissions -> Text,
        description -> Text,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    sessions (id) {
        id -> Int8,
        session_id -> Uuid,
        data -> Text,
        expiry_time -> Timestamptz,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        user_id -> Uuid,
        description -> Text,
        permissions -> Text,
        roles -> Text,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    consumers,
    login_infos,
    merchants,
    password_login_providers,
    permissions,
    roles,
    sessions,
    users,
);

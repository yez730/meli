// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Int8,
        user_id -> Uuid,
        account_id -> Uuid,
        merchant_id -> Uuid,
        cellphone -> Varchar,
        account_name -> Varchar,
        account_state -> Varchar,
        account_photo_url -> Nullable<Text>,
        credential_type -> Nullable<Varchar>,
        credential_no -> Nullable<Varchar>,
        real_name -> Nullable<Varchar>,
        emp_no -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        is_enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::table! {
    consumers (id) {
        id -> Int8,
        user_id -> Uuid,
        consumer_id -> Uuid,
        cellphone -> Varchar,
        real_name -> Nullable<Varchar>,
        credential_type -> Nullable<Varchar>,
        credential_no -> Nullable<Varchar>,
        selfie_photo_url -> Nullable<Text>,
        is_verified -> Bool,
        two_factor_verify -> Bool,
        is_enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::table! {
    merchants (id) {
        id -> Int8,
        merchant_id -> Uuid,
        merchant_name -> Varchar,
        is_enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::table! {
    permissions (id) {
        id -> Int4,
        permission_id -> Uuid,
        permission_code -> Varchar,
        permission_name -> Varchar,
        description -> Text,
        is_enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::table! {
    roles (id) {
        id -> Int4,
        role_id -> Uuid,
        role_code -> Varchar,
        role_name -> Varchar,
        description -> Text,
        is_enabled -> Bool,
        permissions -> Text,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::table! {
    sessions (id) {
        id -> Int8,
        session_id -> Uuid,
        expiry_time -> Timestamptz,
        extra -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        user_id -> Uuid,
        username -> Varchar,
        description -> Text,
        is_enabled -> Bool,
        roles -> Text,
        permissions -> Text,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        extra -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    consumers,
    merchants,
    permissions,
    roles,
    sessions,
    users,
);

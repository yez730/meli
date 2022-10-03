// @generated automatically by Diesel CLI.

diesel::table! {
    barbers (id) {
        id -> Int8,
        user_id -> Uuid,
        barber_id -> Uuid,
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
    login_infos (id) {
        id -> Int8,
        login_info_id -> Uuid,
        login_info_barber -> Varchar,
        login_info_type -> Varchar,
        user_id -> Uuid,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
    }
}

diesel::table! {
    members (id) {
        id -> Int8,
        user_id -> Uuid,
        member_id -> Uuid,
        cellphone -> Varchar,
        real_name -> Nullable<Varchar>,
        gender -> Nullable<Varchar>,
        birth_day -> Nullable<Date>,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
    }
}

diesel::table! {
    merchant_members (id) {
        id -> Int8,
        merchant_id -> Uuid,
        member_id -> Uuid,
        balance -> Numeric,
        enabled -> Bool,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
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
    orders (id) {
        id -> Int8,
        order_id -> Uuid,
        merchant_id -> Uuid,
        date -> Date,
        start_time -> Time,
        end_time -> Time,
        consumer_type -> Varchar,
        member_id -> Nullable<Uuid>,
        barber_id -> Uuid,
        service_type_id -> Uuid,
        status -> Varchar,
        payment_type -> Varchar,
        amount -> Numeric,
        remark -> Nullable<Text>,
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
    recharge_records (id) {
        id -> Int8,
        recharge_record_id -> Uuid,
        merchant_id -> Uuid,
        member_id -> Uuid,
        amount -> Numeric,
        barber_id -> Uuid,
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
    service_types (id) {
        id -> Int8,
        service_type_id -> Uuid,
        merchant_id -> Uuid,
        name -> Varchar,
        normal_prize -> Numeric,
        member_prize -> Numeric,
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
        user_id -> Uuid,
        init_time -> Timestamptz,
        expiry_time -> Timestamptz,
        create_time -> Timestamptz,
        update_time -> Timestamptz,
        data -> Nullable<Text>,
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
    barbers,
    login_infos,
    members,
    merchant_members,
    merchants,
    orders,
    password_login_providers,
    permissions,
    recharge_records,
    roles,
    service_types,
    sessions,
    users,
);

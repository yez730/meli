-- Your SQL goes here

CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_id UUID NOT NULL,
    merchant_id UUID NOT NULL,
    date DATE NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    consumer_type VARCHAR NOT NULL, -- walk-in / member
    member_id UUID NULL,
    barber_id UUID NOT NULL, -- 理发师
    service_type_id UUID NOT NULL,
    status VARCHAR NOT NULL,
    payment_type VARCHAR NOT NULL,
    amount MONEY NOT NULL,
    remark TEXT NULL,
    enabled BOOLEAN NOT NULL, 
    create_time TIMESTAMPTZ NOT NULL,
    update_time TIMESTAMPTZ NOT NULL,
    data TEXT NULL
);

CREATE UNIQUE INDEX orders_order_id_key ON orders
(order_id);

CREATE INDEX orders_merchant_id_idx ON orders
(merchant_id);

CREATE INDEX orders_barber_id_idx ON orders
(barber_id);

CREATE INDEX orders_enabled_idx ON orders
(enabled);

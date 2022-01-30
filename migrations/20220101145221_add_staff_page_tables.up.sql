-- Add up migration script here

CREATE TABLE staff_category (
    id BIGSERIAL PRIMARY KEY,
    "role" VARCHAR NOT NULL UNIQUE,
    ordering BIGSERIAL UNIQUE NOT NULL,
    color INT NOT NULL,
    display_name VARCHAR NOT NULL UNIQUE,
    "text" TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ DEFAULT NULL
);

CREATE TABLE user_staff_entry (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE,
    staff_category_id BIGINT NOT NULL,
    display_name VARCHAR UNIQUE,
    "text" TEXT,
    unavailable BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ DEFAULT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (staff_category_id) REFERENCES staff_category(id)
);
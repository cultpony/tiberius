-- Add up migration script here
CREATE TABLE user_api_keys (
    "id" uuid NOT NULL PRIMARY KEY,
    "user_id" bigint NOT NULL,
    -- Private Key being used in HTTP Auth means API key acts as session for creating user
    "private" varchar NOT NULL,
    -- Key expires at this date
    "valid_until" timestamptz NOT NULL,
    "created_at" timestamptz NOT NULL,
    "updated_at" timestamptz NOT NULL,
    CONSTRAINT fk_user_id FOREIGN KEY (user_id) REFERENCES users(id)
)
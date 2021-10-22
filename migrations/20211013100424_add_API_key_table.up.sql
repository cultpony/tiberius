-- Add up migration script here
CREATE TABLE user_api_keys (
    "id" uuid NOT NULL PRIMARY KEY,
    "user_id" int NOT NULL,
    -- Private Key being used in HTTP Auth means API key acts as session for creating user
    "private" varchar NOT NULL,
    -- Key expires at this date
    "valid_until" timestamp NOT NULL,
    "created_at" timestamp NOT NULL,
    "updated_at" timestamp NOT NULL,
    CONSTRAINT fk_user_id FOREIGN KEY (user_id) REFERENCES users(id)
)
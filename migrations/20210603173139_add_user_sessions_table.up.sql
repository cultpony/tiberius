-- Add migration script here
CREATE TABLE user_sessions (
    "id" uuid NOT NULL PRIMARY KEY,
    "expires" TIMESTAMP WITH TIME ZONE NULL,
    "session" TEXT NOT NULL
)
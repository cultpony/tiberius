-- Add migration script here
CREATE TABLE tag_category (
    name VARCHAR(40) PRIMARY KEY, -- identified by tag category name
    displayname VARCHAR(255), -- shown to user
    description text, -- longtext description of category
    color INT4 -- 32bit colors for tag category
);
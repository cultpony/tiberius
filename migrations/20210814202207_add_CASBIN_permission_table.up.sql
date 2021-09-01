-- Add migration script here
CREATE TABLE IF NOT EXISTS casbin_rule (
    id SERIAL PRIMARY KEY,
    ptype VARCHAR NOT NULL DEFAULT '',
    v0 VARCHAR NOT NULL DEFAULT '',
    v1 VARCHAR NOT NULL DEFAULT '',
    v2 VARCHAR NOT NULL DEFAULT '',
    v3 VARCHAR NOT NULL DEFAULT '',
    v4 VARCHAR NOT NULL DEFAULT '',
    v5 VARCHAR NOT NULL DEFAULT '',
    CONSTRAINT unique_key_sqlx_adapter UNIQUE(ptype, v0, v1, v2, v3, v4, v5)
);

INSERT INTO casbin_rule (ptype, v0, v1) VALUES
('g', 'superuser', 'admin'),
('g', 'admin', 'moderator'),
('g', 'moderator', 'helper'),
('g', 'user::admin@example.com', 'superuser');

INSERT INTO casbin_rule (ptype, v0, v1, v2) VALUES
('p', 'superuser', '*', '*'),
('p', 'admin', '*', '*');
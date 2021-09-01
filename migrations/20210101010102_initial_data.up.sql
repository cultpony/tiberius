BEGIN;
INSERT INTO filters (
    id, "name", "description", 
    "system", "public",
    hidden_complex_str, spoilered_complex_str, hidden_tag_ids, spoilered_tag_ids,
    user_count, created_at, updated_at, user_id
)
VALUES 
(
    1, 'Default', 'The site"s default filter.', TRUE, FALSE, NULL, NULL, ' { 4,7 } ', ' { 3 } ', 0, '2021-05-18 16:02:30 ', '2021-05-18 16:02:30 ', NULL
),
(
    2, 'Everything', 'This filter won"t filter out anything at all.', TRUE, FALSE, NULL, NULL, ' { } ', ' { } ', 0, '2021-05-18 16:02:30 ', '2021-05-18 16:02:30 ', NULL
);

INSERT INTO users
(
    id, email, encrypted_password, sign_in_count, created_at,
    updated_at, authentication_token, name, slug, role
)
VALUES
(
 1, 'admin@example.com', '$2b$12$a0CxBuZrhmfb675NhwiIle3HOBEWCHxWxw4RJOGJel3gimpkRQWQq', 0, '1970-01-01 00:00:00', '1970-01-01 00:00:00',
 'owLZnDjENo9hk9EP',  'Administrator', 'admin', 'admin'
);
 
INSERT INTO forums
(id, name, short_name, description, access_level, created_at, updated_at)
VALUES
(1, 'Site and Policy', 'meta', 'For site discussion and policy discussion', 'normal', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(2, 'Tagging Discussion', 'tagging', 'For discussion regarding site tags, including requesting aliases, implications, spoiler images, and tag descriptions.', 'normal', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(4, 'Site Assistant Discussion', 'helper', 'Restricted - Assistants and Staff', 'assistant', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(5, 'Moderation Discussion', 'mod', 'Restricted - Staff only', 'staff', '1970-01-01 00:00:00', '1970-01-01 00:00:00');

INSERT INTO roles (id, name, resource_id, resource_type, created_at, updated_at)
VALUES
(1, 'moderator', NULL, 'Image', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(2, 'moderator', NULL, 'DuplicateReport', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(3, 'moderator', NULL, 'Comment', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(4, 'moderator', NULL, 'Tag', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(5, 'moderator', NULL, 'UserLink', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(6, 'admin', NULL, 'Tag', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(7, 'moderator', NULL, 'User', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(8, 'admin', NULL, 'SiteNotice', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(9, 'admin', NULL, 'Badge', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(10, 'admin', NULL, 'Role', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(11, 'batch_update', NULL, 'Tag', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(12, 'moderator', NULL, 'Topic', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(13, 'admin', NULL, 'Advert', '1970-01-01 00:00:00', '1970-01-01 00:00:00'),
(14, 'admin', NULL, 'StaticPage', '1970-01-01 00:00:00', '1970-01-01 00:00:00')
;

INSERT INTO users_roles (user_id, role_id) VALUES
(1, 1),
(1, 2),
(1, 3),
(1, 4),
(1, 5),
(1, 6),
(1, 7),
(1, 8),
(1, 9),
(1, 10),
(1, 11),
(1, 12),
(1, 13),
(1, 14);

COMMIT;

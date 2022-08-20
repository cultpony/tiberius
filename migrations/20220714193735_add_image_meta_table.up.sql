-- Add up migration script here
CREATE TABLE images_metadata (
    "id" int NOT NULL UNIQUE,
    views bigint NOT NULL DEFAULT 0,
    CONSTRAINT fk_image_id FOREIGN KEY (id) REFERENCES images(id)
);

INSERT INTO images_metadata (id, views)
(SELECT id, 0 as views from images);
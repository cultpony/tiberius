-- Add up migration script here
CREATE TABLE audit_images (
    id BIGSERIAL PRIMARY KEY,
    image_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    change JSONB NOT NULL,
    reason VARCHAR NOT NULL,
    CONSTRAINT fk_image_id FOREIGN KEY (image_id) REFERENCES images(id),
    CONSTRAINT fk_user_id FOREIGN KEY (user_id) REFERENCES users(id)
)
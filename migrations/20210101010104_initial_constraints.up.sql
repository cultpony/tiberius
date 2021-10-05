ALTER TABLE ONLY image_subscriptions
ADD CONSTRAINT fk_rails_15f6724e1c FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY images
ADD CONSTRAINT fk_rails_19cd822056 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY commissions
ADD CONSTRAINT fk_rails_1cc89d251d FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY badge_awards
ADD CONSTRAINT fk_rails_2bbfd9ee45 FOREIGN KEY (awarded_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY messages
ADD CONSTRAINT fk_rails_2bcf7eed31 FOREIGN KEY (from_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY polls
ADD CONSTRAINT fk_rails_2bf9149369 FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY image_hides
ADD CONSTRAINT fk_rails_335978518a FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY comments
ADD CONSTRAINT fk_rails_33bcaea6cd FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_ips
ADD CONSTRAINT fk_rails_34294629f5 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY channel_subscriptions
ADD CONSTRAINT fk_rails_3447ee7f65 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY commissions
ADD CONSTRAINT fk_rails_3dabda470b FOREIGN KEY (sheet_image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY comments
ADD CONSTRAINT fk_rails_3f25c5a043 FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY unread_notifications
ADD CONSTRAINT fk_rails_429c8d75ab FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY dnp_entries
ADD CONSTRAINT fk_rails_473a736b4a FOREIGN KEY (requesting_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY users_roles
ADD CONSTRAINT fk_rails_4a41696df6 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY tags
ADD CONSTRAINT fk_rails_4b494c6c9a FOREIGN KEY (aliased_tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY conversations
ADD CONSTRAINT fk_rails_4bac0f7b3f FOREIGN KEY (to_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY images
ADD CONSTRAINT fk_rails_4beeabc29a FOREIGN KEY (duplicate_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY mod_notes
ADD CONSTRAINT fk_rails_52f31eb1ff FOREIGN KEY (moderator_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY donations
ADD CONSTRAINT fk_rails_5470822a00 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY commission_items
ADD CONSTRAINT fk_rails_56d368749a FOREIGN KEY (example_image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY posts
ADD CONSTRAINT fk_rails_5736a68073 FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY site_notices
ADD CONSTRAINT fk_rails_57d8d7ea57 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY channel_subscriptions
ADD CONSTRAINT fk_rails_58f2e8e2d4 FOREIGN KEY (channel_id) REFERENCES channels(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY duplicate_reports
ADD CONSTRAINT fk_rails_5b4e8fb78c FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY posts
ADD CONSTRAINT fk_rails_5b5ddfd518 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY duplicate_reports
ADD CONSTRAINT fk_rails_5cf6ede006 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY commission_items
ADD CONSTRAINT fk_rails_62d0ec516b FOREIGN KEY (commission_id) REFERENCES commissions(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY images
ADD CONSTRAINT fk_rails_643b16ae74 FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY topics
ADD CONSTRAINT fk_rails_687ee3cd61 FOREIGN KEY (deleted_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY gallery_interactions
ADD CONSTRAINT fk_rails_6af162285f FOREIGN KEY (gallery_id) REFERENCES galleries(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY galleries
ADD CONSTRAINT fk_rails_6c0cba6a45 FOREIGN KEY (creator_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY gallery_subscriptions
ADD CONSTRAINT fk_rails_6e2d2beaf4 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY posts
ADD CONSTRAINT fk_rails_70d0b6486a FOREIGN KEY (topic_id) REFERENCES topics(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_fingerprints
ADD CONSTRAINT fk_rails_725f1a9b85 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY topic_subscriptions
ADD CONSTRAINT fk_rails_72d9624105 FOREIGN KEY (topic_id) REFERENCES topics(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY duplicate_reports
ADD CONSTRAINT fk_rails_732a84d198 FOREIGN KEY (duplicate_of_image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY galleries
ADD CONSTRAINT fk_rails_792181eb40 FOREIGN KEY (thumbnail_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY image_hides
ADD CONSTRAINT fk_rails_7a10a4b0f1 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY topics
ADD CONSTRAINT fk_rails_7b812cfb44 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY image_votes
ADD CONSTRAINT fk_rails_8086a2c07e FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY forum_subscriptions
ADD CONSTRAINT fk_rails_8268bd8830 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_name_changes
ADD CONSTRAINT fk_rails_828a40cab1 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY poll_votes
ADD CONSTRAINT fk_rails_848ece0184 FOREIGN KEY (poll_option_id) REFERENCES poll_options(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY forum_subscriptions
ADD CONSTRAINT fk_rails_8508ff98b6 FOREIGN KEY (forum_id) REFERENCES forums(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY polls
ADD CONSTRAINT fk_rails_861a79e923 FOREIGN KEY (topic_id) REFERENCES topics(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY topics
ADD CONSTRAINT fk_rails_8fdcbf6aed FOREIGN KEY (last_post_id) REFERENCES posts(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY image_features
ADD CONSTRAINT fk_rails_90c2421c89 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY unread_notifications
ADD CONSTRAINT fk_rails_97681c85bb FOREIGN KEY (notification_id) REFERENCES notifications(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_links
ADD CONSTRAINT fk_rails_9939489c5c FOREIGN KEY (verified_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY fingerprint_bans
ADD CONSTRAINT fk_rails_9a0218c560 FOREIGN KEY (banning_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY users
ADD CONSTRAINT fk_rails_9efba9a459 FOREIGN KEY (deleted_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY user_statistics
ADD CONSTRAINT fk_rails_a4ae2a454b FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY image_subscriptions
ADD CONSTRAINT fk_rails_a4ee3b390b FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY forums
ADD CONSTRAINT fk_rails_a63558903d FOREIGN KEY (last_post_id) REFERENCES posts(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY poll_options
ADD CONSTRAINT fk_rails_aa85becb42 FOREIGN KEY (poll_id) REFERENCES polls(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_links
ADD CONSTRAINT fk_rails_ab45cd8fd7 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY topics
ADD CONSTRAINT fk_rails_ab6fa5b2e7 FOREIGN KEY (locked_by_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY topic_subscriptions
ADD CONSTRAINT fk_rails_b0d5d379ae FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY reports
ADD CONSTRAINT fk_rails_b138baacff FOREIGN KEY (admin_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY user_bans
ADD CONSTRAINT fk_rails_b27db52384 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY static_page_versions
ADD CONSTRAINT fk_rails_b3d9f91a2b FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY image_features
ADD CONSTRAINT fk_rails_b5fb903247 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY poll_votes
ADD CONSTRAINT fk_rails_b64de9b025 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY tags_implied_tags
ADD CONSTRAINT fk_rails_b70078b5dd FOREIGN KEY (implied_tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY image_intensities
ADD CONSTRAINT fk_rails_b861f027a7 FOREIGN KEY (image_id) REFERENCES images(id);
ALTER TABLE ONLY badge_awards
ADD CONSTRAINT fk_rails_b95340cf70 FOREIGN KEY (badge_id) REFERENCES badges(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY gallery_interactions
ADD CONSTRAINT fk_rails_bb5ebe2a77 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY image_faves
ADD CONSTRAINT fk_rails_bebe1c640a FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY static_page_versions
ADD CONSTRAINT fk_rails_bfb173af6a FOREIGN KEY (static_page_id) REFERENCES static_pages(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY image_votes
ADD CONSTRAINT fk_rails_c6d2f46f70 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY reports
ADD CONSTRAINT fk_rails_c7699d537d FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY conversations
ADD CONSTRAINT fk_rails_d0f47f4937 FOREIGN KEY (from_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY duplicate_reports
ADD CONSTRAINT fk_rails_d209e0f2ed FOREIGN KEY (modifier_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY users
ADD CONSTRAINT fk_rails_d2b4c2768f FOREIGN KEY (current_filter_id) REFERENCES filters(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY user_bans
ADD CONSTRAINT fk_rails_d4cf1d1b70 FOREIGN KEY (banning_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY subnet_bans
ADD CONSTRAINT fk_rails_d8a07ba049 FOREIGN KEY (banning_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE ONLY dnp_entries
ADD CONSTRAINT fk_rails_df26188cea FOREIGN KEY (modifying_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY tags_implied_tags
ADD CONSTRAINT fk_rails_e55707c39a FOREIGN KEY (tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_links
ADD CONSTRAINT fk_rails_e6cf0175d0 FOREIGN KEY (contacted_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY forums
ADD CONSTRAINT fk_rails_e8afa7749e FOREIGN KEY (last_topic_id) REFERENCES topics(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE ONLY topics
ADD CONSTRAINT fk_rails_eac66eb971 FOREIGN KEY (forum_id) REFERENCES forums(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY users_roles
ADD CONSTRAINT fk_rails_eb7b4658f8 FOREIGN KEY (role_id) REFERENCES roles(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_whitelists
ADD CONSTRAINT fk_rails_eda0eaebbb FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY dnp_entries
ADD CONSTRAINT fk_rails_f428aa5665 FOREIGN KEY (tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY filters
ADD CONSTRAINT fk_rails_f53aed9bb6 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY user_links
ADD CONSTRAINT fk_rails_f64b4291c0 FOREIGN KEY (tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY gallery_subscriptions
ADD CONSTRAINT fk_rails_fa77f3cebe FOREIGN KEY (gallery_id) REFERENCES galleries(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE ONLY image_sources
ADD CONSTRAINT image_sources_image_id_fkey FOREIGN KEY (image_id) REFERENCES images(id);
ALTER TABLE ONLY user_tokens
ADD CONSTRAINT user_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE ONLY users
ADD CONSTRAINT users_forced_filter_id_fkey FOREIGN KEY (forced_filter_id) REFERENCES filters(id);
ALTER TABLE tag_changes
ADD CONSTRAINT fk_rails_0e6c53f1b9 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE,
    ADD CONSTRAINT fk_rails_1d7b844de4 FOREIGN KEY (tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE
SET NULL,
    ADD CONSTRAINT fk_rails_82fc2dd958 FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE source_changes
ADD CONSTRAINT fk_rails_8d8cb9cb3b FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET NULL,
    ADD CONSTRAINT fk_rails_10271ec4d0 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE comments
ADD CONSTRAINT fk_rails_03de2dc08c FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE
SET null;
ALTER TABLE badge_awards
ADD CONSTRAINT fk_rails_0434c93bfb FOREIGN KEY (user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE channels
ADD CONSTRAINT fk_rails_021c624081 FOREIGN KEY (associated_artist_tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE
SET NULL;
ALTER TABLE image_Faves
ADD CONSTRAINT fk_rails_0a4bb301d6 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE image_taggings
ADD CONSTRAINT fk_rails_74cc21a055 FOREIGN KEY (tag_id) REFERENCES tags(id) ON UPDATE CASCADE ON DELETE CASCADE,
    ADD CONSTRAINT fk_rails_0f89cd23a9 FOREIGN KEY (image_id) REFERENCES images(id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE image_sources
ADD CONSTRAINT length_must_be_valid CHECK (
        (
            (length(source) >= 8)
            AND (length(source) <= 1024)
        )
    );
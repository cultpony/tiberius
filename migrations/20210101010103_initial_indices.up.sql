CREATE INDEX image_intensities_index ON image_intensities USING btree (nw, ne, sw, se);
CREATE UNIQUE INDEX image_sources_image_id_source_index ON image_sources USING btree (image_id, source);
CREATE INDEX index_adverts_on_restrictions ON adverts USING btree (restrictions);
CREATE INDEX index_adverts_on_start_date_and_finish_date ON adverts USING btree (start_date, finish_date);
CREATE INDEX index_badge_awards_on_awarded_by_id ON badge_awards USING btree (awarded_by_id);
CREATE INDEX index_badge_awards_on_badge_id ON badge_awards USING btree (badge_id);
CREATE INDEX index_badge_awards_on_user_id ON badge_awards USING btree (user_id);
CREATE UNIQUE INDEX index_channel_subscriptions_on_channel_id_and_user_id ON channel_subscriptions USING btree (channel_id, user_id);
CREATE INDEX index_channel_subscriptions_on_user_id ON channel_subscriptions USING btree (user_id);
CREATE INDEX index_channels_on_associated_artist_tag_id ON channels USING btree (associated_artist_tag_id);
CREATE INDEX index_channels_on_is_live ON channels USING btree (is_live);
CREATE INDEX index_channels_on_last_fetched_at ON channels USING btree (last_fetched_at);
CREATE INDEX index_channels_on_next_check_at ON channels USING btree (next_check_at);
CREATE INDEX index_comments_on_created_at ON comments USING btree (created_at);
CREATE INDEX index_comments_on_deleted_by_id ON comments USING btree (deleted_by_id)
WHERE (deleted_by_id IS NOT NULL);
CREATE INDEX index_comments_on_image_id ON comments USING btree (image_id);
CREATE INDEX index_comments_on_image_id_and_created_at ON comments USING btree (image_id, created_at);
CREATE INDEX index_comments_on_user_id ON comments USING btree (user_id);
CREATE INDEX index_commission_items_on_commission_id ON commission_items USING btree (commission_id);
CREATE INDEX index_commission_items_on_example_image_id ON commission_items USING btree (example_image_id);
CREATE INDEX index_commission_items_on_item_type ON commission_items USING btree (item_type);
CREATE INDEX index_commissions_on_open ON commissions USING btree (open);
CREATE INDEX index_commissions_on_sheet_image_id ON commissions USING btree (sheet_image_id);
CREATE INDEX index_commissions_on_user_id ON commissions USING btree (user_id);
CREATE INDEX index_conversations_on_created_at_and_from_hidden ON conversations USING btree (created_at, from_hidden);
CREATE INDEX index_conversations_on_from_id ON conversations USING btree (from_id);
CREATE INDEX index_conversations_on_to_id ON conversations USING btree (to_id);
CREATE INDEX index_dnp_entries_on_aasm_state_filtered ON dnp_entries USING btree (aasm_state)
WHERE (
        (aasm_state)::text = ANY (
            ARRAY [('requested'::character varying)::text, ('claimed'::character varying)::text, ('rescinded'::character varying)::text, ('acknowledged'::character varying)::text]
        )
    );
CREATE INDEX index_dnp_entries_on_requesting_user_id ON dnp_entries USING btree (requesting_user_id);
CREATE INDEX index_dnp_entries_on_tag_id ON dnp_entries USING btree (tag_id);
CREATE INDEX index_donations_on_user_id ON donations USING btree (user_id);
CREATE INDEX index_duplicate_reports_on_created_at ON duplicate_reports USING btree (created_at);
CREATE INDEX index_duplicate_reports_on_duplicate_of_image_id ON duplicate_reports USING btree (duplicate_of_image_id);
CREATE INDEX index_duplicate_reports_on_image_id ON duplicate_reports USING btree (image_id);
CREATE INDEX index_duplicate_reports_on_modifier_id ON duplicate_reports USING btree (modifier_id);
CREATE INDEX index_duplicate_reports_on_state ON duplicate_reports USING btree (state);
CREATE INDEX index_duplicate_reports_on_state_filtered ON duplicate_reports USING btree (state)
WHERE (
        (state)::text = ANY (
            ARRAY [('open'::character varying)::text, ('claimed'::character varying)::text]
        )
    );
CREATE INDEX index_duplicate_reports_on_user_id ON duplicate_reports USING btree (user_id);
CREATE INDEX index_filters_on_name ON filters USING btree (name);
CREATE INDEX index_filters_on_system ON filters USING btree (system)
WHERE (system = true);
CREATE INDEX index_filters_on_user_id ON filters USING btree (user_id);
CREATE INDEX index_fingerprint_bans_on_banning_user_id ON fingerprint_bans USING btree (banning_user_id);
CREATE INDEX index_fingerprint_bans_on_created_at ON fingerprint_bans USING btree (created_at);
CREATE INDEX index_fingerprint_bans_on_fingerprint ON fingerprint_bans USING btree (fingerprint);
CREATE UNIQUE INDEX index_forum_subscriptions_on_forum_id_and_user_id ON forum_subscriptions USING btree (forum_id, user_id);
CREATE INDEX index_forum_subscriptions_on_user_id ON forum_subscriptions USING btree (user_id);
CREATE INDEX index_forums_on_last_post_id ON forums USING btree (last_post_id);
CREATE INDEX index_forums_on_last_topic_id ON forums USING btree (last_topic_id);
CREATE UNIQUE INDEX index_forums_on_short_name ON forums USING btree (short_name);
CREATE INDEX index_galleries_on_creator_id ON galleries USING btree (creator_id);
CREATE INDEX index_galleries_on_thumbnail_id ON galleries USING btree (thumbnail_id);
CREATE INDEX index_gallery_interactions_on_gallery_id ON gallery_interactions USING btree (gallery_id);
CREATE UNIQUE INDEX index_gallery_interactions_on_gallery_id_and_image_id ON gallery_interactions USING btree (gallery_id, image_id);
CREATE INDEX index_gallery_interactions_on_gallery_id_and_position ON gallery_interactions USING btree (gallery_id, "position");
CREATE INDEX index_gallery_interactions_on_image_id ON gallery_interactions USING btree (image_id);
CREATE INDEX index_gallery_interactions_on_position ON gallery_interactions USING btree ("position");
CREATE UNIQUE INDEX index_gallery_subscriptions_on_gallery_id_and_user_id ON gallery_subscriptions USING btree (gallery_id, user_id);
CREATE INDEX index_gallery_subscriptions_on_user_id ON gallery_subscriptions USING btree (user_id);
CREATE UNIQUE INDEX index_image_faves_on_image_id_and_user_id ON image_faves USING btree (image_id, user_id);
CREATE INDEX index_image_faves_on_user_id ON image_faves USING btree (user_id);
CREATE INDEX index_image_features_on_created_at ON image_features USING btree (created_at);
CREATE INDEX index_image_features_on_image_id ON image_features USING btree (image_id);
CREATE INDEX index_image_features_on_user_id ON image_features USING btree (user_id);
CREATE UNIQUE INDEX index_image_hides_on_image_id_and_user_id ON image_hides USING btree (image_id, user_id);
CREATE INDEX index_image_hides_on_user_id ON image_hides USING btree (user_id);
CREATE UNIQUE INDEX index_image_intensities_on_image_id ON image_intensities USING btree (image_id);
CREATE UNIQUE INDEX index_image_subscriptions_on_image_id_and_user_id ON image_subscriptions USING btree (image_id, user_id);
CREATE INDEX index_image_subscriptions_on_user_id ON image_subscriptions USING btree (user_id);
CREATE UNIQUE INDEX index_image_taggings_on_image_id_and_tag_id ON image_taggings USING btree (image_id, tag_id);
CREATE INDEX index_image_taggings_on_tag_id ON image_taggings USING btree (tag_id);
CREATE UNIQUE INDEX index_image_votes_on_image_id_and_user_id ON image_votes USING btree (image_id, user_id);
CREATE INDEX index_image_votes_on_user_id ON image_votes USING btree (user_id);
CREATE INDEX index_images_on_created_at ON images USING btree (created_at);
CREATE INDEX index_images_on_deleted_by_id ON images USING btree (deleted_by_id)
WHERE (deleted_by_id IS NOT NULL);
CREATE INDEX index_images_on_duplicate_id ON images USING btree (duplicate_id)
WHERE (duplicate_id IS NOT NULL);
CREATE INDEX index_images_on_featured_on ON images USING btree (featured_on);
CREATE INDEX index_images_on_image_orig_sha512_hash ON images USING btree (image_orig_sha512_hash);
CREATE INDEX index_images_on_tag_ids ON images USING gin (tag_ids);
CREATE INDEX index_images_on_updated_at ON images USING btree (updated_at);
CREATE INDEX index_images_on_user_id ON images USING btree (user_id);
CREATE INDEX index_messages_on_conversation_id_and_created_at ON messages USING btree (conversation_id, created_at);
CREATE INDEX index_messages_on_from_id ON messages USING btree (from_id);
CREATE INDEX index_mod_notes_on_moderator_id ON mod_notes USING btree (moderator_id);
CREATE INDEX index_mod_notes_on_notable_type_and_notable_id ON mod_notes USING btree (notable_type, notable_id);
CREATE UNIQUE INDEX index_notifications_on_actor_id_and_actor_type ON notifications USING btree (actor_id, actor_type);
CREATE UNIQUE INDEX index_poll_options_on_poll_id_and_label ON poll_options USING btree (poll_id, label);
CREATE UNIQUE INDEX index_poll_votes_on_poll_option_id_and_user_id ON poll_votes USING btree (poll_option_id, user_id);
CREATE INDEX index_poll_votes_on_user_id ON poll_votes USING btree (user_id);
CREATE INDEX index_polls_on_deleted_by_id ON polls USING btree (deleted_by_id)
WHERE (deleted_by_id IS NOT NULL);
CREATE INDEX index_polls_on_topic_id ON polls USING btree (topic_id);
CREATE INDEX index_posts_on_deleted_by_id ON posts USING btree (deleted_by_id)
WHERE (deleted_by_id IS NOT NULL);
CREATE INDEX index_posts_on_topic_id_and_created_at ON posts USING btree (topic_id, created_at);
CREATE INDEX index_posts_on_topic_id_and_topic_position ON posts USING btree (topic_id, topic_position);
CREATE INDEX index_posts_on_user_id ON posts USING btree (user_id);
CREATE INDEX index_reports_on_admin_id ON reports USING btree (admin_id);
CREATE INDEX index_reports_on_created_at ON reports USING btree (created_at);
CREATE INDEX index_reports_on_open ON reports USING btree (open);
CREATE INDEX index_reports_on_user_id ON reports USING btree (user_id);
CREATE INDEX index_roles_on_name_and_resource_type_and_resource_id ON roles USING btree (name, resource_type, resource_id);
CREATE INDEX index_site_notices_on_start_date_and_finish_date ON site_notices USING btree (start_date, finish_date);
CREATE INDEX index_site_notices_on_user_id ON site_notices USING btree (user_id);
CREATE INDEX index_source_changes_on_image_id ON source_changes USING btree (image_id);
CREATE INDEX index_source_changes_on_ip ON source_changes USING btree (ip);
CREATE INDEX index_source_changes_on_user_id ON source_changes USING btree (user_id);
CREATE INDEX index_static_page_versions_on_static_page_id ON static_page_versions USING btree (static_page_id);
CREATE INDEX index_static_page_versions_on_user_id ON static_page_versions USING btree (user_id);
CREATE UNIQUE INDEX index_static_pages_on_slug ON static_pages USING btree (slug);
CREATE UNIQUE INDEX index_static_pages_on_title ON static_pages USING btree (title);
CREATE INDEX index_subnet_bans_on_banning_user_id ON subnet_bans USING btree (banning_user_id);
CREATE INDEX index_subnet_bans_on_created_at ON subnet_bans USING btree (created_at);
CREATE INDEX index_subnet_bans_on_specification ON subnet_bans USING gist (specification inet_ops);
CREATE INDEX index_tag_changes_on_fingerprint ON tag_changes USING btree (fingerprint);
CREATE INDEX index_tag_changes_on_image_id ON tag_changes USING btree (image_id);
CREATE INDEX index_tag_changes_on_ip ON tag_changes USING gist (ip inet_ops);
CREATE INDEX index_tag_changes_on_tag_id ON tag_changes USING btree (tag_id);
CREATE INDEX index_tag_changes_on_user_id ON tag_changes USING btree (user_id);
CREATE INDEX index_tags_implied_tags_on_implied_tag_id ON tags_implied_tags USING btree (implied_tag_id);
CREATE UNIQUE INDEX index_tags_implied_tags_on_tag_id_and_implied_tag_id ON tags_implied_tags USING btree (tag_id, implied_tag_id);
CREATE INDEX index_tags_on_aliased_tag_id ON tags USING btree (aliased_tag_id);
CREATE UNIQUE INDEX index_tags_on_name ON tags USING btree (name);
CREATE UNIQUE INDEX index_tags_on_slug ON tags USING btree (slug);
CREATE UNIQUE INDEX index_topic_subscriptions_on_topic_id_and_user_id ON topic_subscriptions USING btree (topic_id, user_id);
CREATE INDEX index_topic_subscriptions_on_user_id ON topic_subscriptions USING btree (user_id);
CREATE INDEX index_topics_on_deleted_by_id ON topics USING btree (deleted_by_id)
WHERE (deleted_by_id IS NOT NULL);
CREATE INDEX index_topics_on_forum_id ON topics USING btree (forum_id);
CREATE UNIQUE INDEX index_topics_on_forum_id_and_slug ON topics USING btree (forum_id, slug);
CREATE INDEX index_topics_on_hidden_from_users ON topics USING btree (hidden_from_users);
CREATE INDEX index_topics_on_last_post_id ON topics USING btree (last_post_id);
CREATE INDEX index_topics_on_last_replied_to_at ON topics USING btree (last_replied_to_at);
CREATE INDEX index_topics_on_locked_by_id ON topics USING btree (locked_by_id)
WHERE (locked_by_id IS NOT NULL);
CREATE INDEX index_topics_on_slug ON topics USING btree (slug);
CREATE INDEX index_topics_on_user_id ON topics USING btree (user_id);
CREATE UNIQUE INDEX index_unread_notifications_on_notification_id_and_user_id ON unread_notifications USING btree (notification_id, user_id);
CREATE INDEX index_unread_notifications_on_user_id ON unread_notifications USING btree (user_id);
CREATE INDEX index_user_bans_on_banning_user_id ON user_bans USING btree (banning_user_id);
CREATE INDEX index_user_bans_on_created_at ON user_bans USING btree (created_at DESC);
CREATE INDEX index_user_bans_on_user_id ON user_bans USING btree (user_id);
CREATE UNIQUE INDEX index_user_fingerprints_on_fingerprint_and_user_id ON user_fingerprints USING btree (fingerprint, user_id);
CREATE INDEX index_user_fingerprints_on_user_id ON user_fingerprints USING btree (user_id);
CREATE UNIQUE INDEX index_user_ips_on_ip_and_user_id ON user_ips USING btree (ip, user_id);
CREATE INDEX index_user_ips_on_updated_at ON user_ips USING btree (updated_at);
CREATE INDEX index_user_ips_on_user_id_and_updated_at ON user_ips USING btree (user_id, updated_at DESC);
CREATE INDEX index_user_links_on_aasm_state ON user_links USING btree (aasm_state);
CREATE INDEX index_user_links_on_contacted_by_user_id ON user_links USING btree (contacted_by_user_id);
CREATE INDEX index_user_links_on_next_check_at ON user_links USING btree (next_check_at);
CREATE INDEX index_user_links_on_tag_id ON user_links USING btree (tag_id);
CREATE UNIQUE INDEX index_user_links_on_uri_tag_id_user_id ON user_links USING btree (uri, tag_id, user_id)
WHERE ((aasm_state)::text <> 'rejected'::text);
CREATE INDEX index_user_links_on_user_id ON user_links USING btree (user_id);
CREATE INDEX index_user_links_on_verified_by_user_id ON user_links USING btree (verified_by_user_id);
CREATE INDEX index_user_name_changes_on_user_id ON user_name_changes USING btree (user_id);
CREATE INDEX index_user_statistics_on_user_id ON user_statistics USING btree (user_id);
CREATE UNIQUE INDEX index_user_statistics_on_user_id_and_day ON user_statistics USING btree (user_id, day);
CREATE UNIQUE INDEX index_user_whitelists_on_user_id ON user_whitelists USING btree (user_id);
CREATE UNIQUE INDEX index_users_on_authentication_token ON users USING btree (authentication_token);
CREATE INDEX index_users_on_created_at ON users USING btree (created_at);
CREATE INDEX index_users_on_current_filter_id ON users USING btree (current_filter_id);
CREATE INDEX index_users_on_deleted_by_user_id ON users USING btree (deleted_by_user_id)
WHERE (deleted_by_user_id IS NOT NULL);
CREATE UNIQUE INDEX index_users_on_email ON users USING btree (email);
CREATE UNIQUE INDEX index_users_on_name ON users USING btree (name);
CREATE UNIQUE INDEX index_users_on_reset_password_token ON users USING btree (reset_password_token);
CREATE INDEX index_users_on_role ON users USING btree (role)
WHERE ((role)::text <> 'user'::text);
CREATE UNIQUE INDEX index_users_on_slug ON users USING btree (slug);
CREATE INDEX index_users_on_watched_tag_ids ON users USING gin (watched_tag_ids);
CREATE INDEX index_users_roles_on_role_id ON users_roles USING btree (role_id);
CREATE UNIQUE INDEX index_users_roles_on_user_id_and_role_id ON users_roles USING btree (user_id, role_id);
CREATE INDEX index_versions_on_item_type_and_item_id ON versions USING btree (item_type, item_id);
CREATE INDEX index_vpns_on_ip ON vpns USING gist (ip inet_ops);
CREATE INDEX intensities_index ON images USING btree (
    se_intensity,
    sw_intensity,
    ne_intensity,
    nw_intensity,
    average_intensity
);
CREATE UNIQUE INDEX user_tokens_context_token_index ON user_tokens USING btree (context, token);
CREATE INDEX user_tokens_user_id_index ON user_tokens USING btree (user_id);
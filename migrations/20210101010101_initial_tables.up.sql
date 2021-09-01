CREATE EXTENSION IF NOT EXISTS citext;
CREATE TABLE adverts (
    id SERIAL PRIMARY KEY,
    image character varying,
    link character varying,
    title character varying,
    clicks integer DEFAULT 0,
    impressions integer DEFAULT 0,
    live boolean DEFAULT false,
    start_date timestamp without time zone,
    finish_date timestamp without time zone,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    restrictions character varying,
    notes character varying
);
CREATE TABLE badges (
    id SERIAL PRIMARY KEY,
    title character varying NOT NULL,
    description character varying NOT NULL,
    image character varying,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    disable_award boolean DEFAULT false NOT NULL,
    priority boolean DEFAULT false
);
CREATE TABLE channel_subscriptions (
    channel_id integer NOT NULL,
    user_id integer NOT NULL,
    PRIMARY KEY (channel_id, user_id)
);
CREATE TABLE channels (
    id SERIAL PRIMARY KEY,
    short_name character varying NOT NULL,
    title character varying NOT NULL,
    description character varying,
    channel_image character varying,
    tags character varying,
    viewers integer DEFAULT 0 NOT NULL,
    nsfw boolean DEFAULT false NOT NULL,
    is_live boolean DEFAULT false NOT NULL,
    last_fetched_at timestamp without time zone,
    next_check_at timestamp without time zone,
    last_live_at timestamp without time zone,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watcher_count integer DEFAULT 0 NOT NULL,
    type character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    associated_artist_tag_id integer,
    viewer_minutes_today integer DEFAULT 0 NOT NULL,
    viewer_minutes_thisweek integer DEFAULT 0 NOT NULL,
    viewer_minutes_thismonth integer DEFAULT 0 NOT NULL,
    total_viewer_minutes integer DEFAULT 0 NOT NULL,
    banner_image character varying,
    remote_stream_id integer,
    thumbnail_url character varying DEFAULT ''::character varying
);
CREATE TABLE commission_items (
    id SERIAL PRIMARY KEY,
    commission_id integer,
    item_type character varying,
    description character varying,
    base_price numeric,
    add_ons character varying,
    example_image_id integer,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL
);
CREATE TABLE commissions (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    open boolean NOT NULL,
    categories character varying [] DEFAULT '{}'::character varying [] NOT NULL,
    information character varying,
    contact character varying,
    sheet_image_id integer,
    will_create character varying,
    will_not_create character varying,
    commission_items_count integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL
);
CREATE TABLE conversations (
    id SERIAL PRIMARY KEY,
    title character varying NOT NULL,
    to_read boolean DEFAULT false NOT NULL,
    from_read boolean DEFAULT true NOT NULL,
    to_hidden boolean DEFAULT false NOT NULL,
    from_hidden boolean DEFAULT false NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    from_id integer NOT NULL,
    to_id integer NOT NULL,
    slug character varying NOT NULL,
    last_message_at timestamp without time zone NOT NULL
);
CREATE TABLE dnp_entries (
    id SERIAL PRIMARY KEY,
    requesting_user_id integer NOT NULL,
    modifying_user_id integer,
    tag_id integer NOT NULL,
    aasm_state character varying DEFAULT 'requested'::character varying NOT NULL,
    dnp_type character varying NOT NULL,
    conditions character varying NOT NULL,
    reason character varying NOT NULL,
    hide_reason boolean DEFAULT false NOT NULL,
    instructions character varying NOT NULL,
    feedback character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL
);
CREATE TABLE donations (
    id SERIAL PRIMARY KEY,
    email character varying,
    amount numeric,
    fee numeric,
    txn_id character varying,
    receipt_id character varying,
    note character varying,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer
);
CREATE TABLE duplicate_reports (
    id SERIAL PRIMARY KEY,
    reason character varying,
    state character varying DEFAULT 'open'::character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    image_id integer NOT NULL,
    duplicate_of_image_id integer NOT NULL,
    user_id integer,
    modifier_id integer
);
CREATE TABLE filters (
    id SERIAL PRIMARY KEY,
    name character varying NOT NULL,
    description character varying NOT NULL,
    system boolean DEFAULT false NOT NULL,
    public boolean DEFAULT false NOT NULL,
    hidden_complex_str character varying,
    spoilered_complex_str character varying,
    hidden_tag_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    spoilered_tag_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    user_count integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer
);
CREATE TABLE fingerprint_bans (
    id SERIAL PRIMARY KEY,
    reason character varying NOT NULL,
    note character varying,
    enabled boolean DEFAULT true NOT NULL,
    valid_until timestamp without time zone NOT NULL,
    fingerprint character varying,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    banning_user_id integer NOT NULL,
    generated_ban_id character varying NOT NULL
);
CREATE TABLE forum_subscriptions (
    forum_id integer NOT NULL,
    user_id integer NOT NULL,
    PRIMARY KEY (forum_id, user_id)
);
CREATE TABLE forums (
    id SERIAL PRIMARY KEY,
    name character varying NOT NULL,
    short_name character varying NOT NULL,
    description character varying NOT NULL,
    access_level character varying DEFAULT 'normal'::character varying NOT NULL,
    topic_count integer DEFAULT 0 NOT NULL,
    post_count integer DEFAULT 0 NOT NULL,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watcher_count integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    last_post_id integer,
    last_topic_id integer
);
CREATE TABLE galleries (
    id SERIAL PRIMARY KEY,
    title character varying NOT NULL,
    spoiler_warning character varying DEFAULT ''::character varying NOT NULL,
    description character varying DEFAULT ''::character varying NOT NULL,
    thumbnail_id integer NOT NULL,
    creator_id integer NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watcher_count integer DEFAULT 0 NOT NULL,
    image_count integer DEFAULT 0 NOT NULL,
    order_position_asc boolean DEFAULT false NOT NULL
);
CREATE TABLE gallery_interactions (
    id SERIAL PRIMARY KEY,
    "position" integer NOT NULL,
    image_id integer NOT NULL,
    gallery_id integer NOT NULL
);
CREATE TABLE gallery_subscriptions (
    gallery_id integer NOT NULL,
    user_id integer NOT NULL,
    PRIMARY KEY (gallery_id, user_id)
);
CREATE TABLE image_faves (
    image_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    PRIMARY KEY (image_id, user_id)
);
CREATE TABLE image_features (
    id bigserial PRIMARY KEY,
    image_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL
);
CREATE TABLE image_hides (
    image_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    PRIMARY KEY (image_id, user_id)
);
CREATE TABLE image_intensities (
    id bigint PRIMARY KEY,
    image_id bigint NOT NULL,
    nw double precision NOT NULL,
    ne double precision NOT NULL,
    sw double precision NOT NULL,
    se double precision NOT NULL
);
CREATE TABLE image_sources (
    id bigint PRIMARY KEY,
    image_id bigint NOT NULL,
    source text NOT NULL
);
CREATE TABLE image_subscriptions (
    image_id integer NOT NULL,
    user_id integer NOT NULL,
    PRIMARY KEY (image_id, user_id)
);
CREATE TABLE image_taggings (
    image_id bigint NOT NULL,
    tag_id bigint NOT NULL,
    PRIMARY KEY (image_id, tag_id)
);
CREATE TABLE image_votes (
    image_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamp without time zone NOT NULL,
    up boolean NOT NULL,
    PRIMARY KEY (image_id, user_id)
);
CREATE TABLE images (
    id SERIAL PRIMARY KEY,
    image character varying,
    image_name character varying,
    image_width integer,
    image_height integer,
    image_size integer,
    image_format character varying,
    image_mime_type character varying,
    image_aspect_ratio double precision,
    ip inet,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    anonymous boolean DEFAULT false,
    score integer DEFAULT 0 NOT NULL,
    faves_count integer DEFAULT 0 NOT NULL,
    upvotes_count integer DEFAULT 0 NOT NULL,
    downvotes_count integer DEFAULT 0 NOT NULL,
    votes_count integer DEFAULT 0 NOT NULL,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watcher_count integer DEFAULT 0 NOT NULL,
    source_url character varying,
    description character varying DEFAULT ''::character varying NOT NULL,
    image_sha512_hash character varying,
    image_orig_sha512_hash character varying,
    deletion_reason character varying,
    tag_list_cache character varying,
    tag_list_plus_alias_cache character varying,
    file_name_cache character varying,
    duplicate_id integer,
    tag_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    comments_count integer DEFAULT 0 NOT NULL,
    processed boolean DEFAULT false NOT NULL,
    thumbnails_generated boolean DEFAULT false NOT NULL,
    duplication_checked boolean DEFAULT false NOT NULL,
    hidden_from_users boolean DEFAULT false NOT NULL,
    tag_editing_allowed boolean DEFAULT true NOT NULL,
    description_editing_allowed boolean DEFAULT true NOT NULL,
    commenting_allowed boolean DEFAULT true NOT NULL,
    is_animated boolean NOT NULL,
    first_seen_at timestamp without time zone NOT NULL,
    featured_on timestamp without time zone,
    se_intensity double precision,
    sw_intensity double precision,
    ne_intensity double precision,
    nw_intensity double precision,
    average_intensity double precision,
    user_id integer,
    deleted_by_id integer,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    destroyed_content boolean DEFAULT false NOT NULL,
    hidden_image_key character varying,
    scratchpad character varying,
    hides_count integer DEFAULT 0 NOT NULL,
    image_duration double precision
);
CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    body character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    from_id integer NOT NULL,
    conversation_id integer NOT NULL
);
CREATE TABLE mod_notes (
    id SERIAL PRIMARY KEY,
    moderator_id integer NOT NULL,
    notable_id integer NOT NULL,
    notable_type character varying NOT NULL,
    body text NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL
);
CREATE TABLE notifications (
    id SERIAL PRIMARY KEY,
    action character varying NOT NULL,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    actor_id integer NOT NULL,
    actor_type character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    actor_child_id integer,
    actor_child_type character varying
);
CREATE TABLE poll_options (
    id SERIAL PRIMARY KEY,
    label character varying(80) NOT NULL,
    vote_count integer DEFAULT 0 NOT NULL,
    poll_id integer NOT NULL
);
CREATE TABLE poll_votes (
    id SERIAL PRIMARY KEY,
    rank integer,
    poll_option_id integer NOT NULL,
    user_id integer NOT NULL,
    created_at timestamp without time zone NOT NULL
);
CREATE TABLE polls (
    id SERIAL PRIMARY KEY,
    title character varying(140) NOT NULL,
    vote_method character varying(8) NOT NULL,
    active_until timestamp without time zone NOT NULL,
    total_votes integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    hidden_from_users boolean DEFAULT false NOT NULL,
    deleted_by_id integer,
    deletion_reason character varying DEFAULT ''::character varying NOT NULL,
    topic_id integer NOT NULL
);
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    body character varying NOT NULL,
    edit_reason character varying,
    ip inet,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    topic_position integer NOT NULL,
    hidden_from_users boolean DEFAULT false NOT NULL,
    anonymous boolean DEFAULT false,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer,
    topic_id integer NOT NULL,
    deleted_by_id integer,
    edited_at timestamp without time zone,
    deletion_reason character varying DEFAULT ''::character varying NOT NULL,
    destroyed_content boolean DEFAULT false NOT NULL,
    name_at_post_time character varying
);
CREATE TABLE reports (
    id SERIAL PRIMARY KEY,
    ip inet NOT NULL,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    reason character varying NOT NULL,
    state character varying DEFAULT 'open'::character varying NOT NULL,
    open boolean DEFAULT true NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer,
    admin_id integer,
    reportable_id integer NOT NULL,
    reportable_type character varying NOT NULL
);
CREATE TABLE roles (
    id SERIAL PRIMARY KEY,
    name character varying,
    resource_id integer,
    resource_type character varying,
    created_at timestamp without time zone,
    updated_at timestamp without time zone
);
CREATE TABLE site_notices (
    id SERIAL PRIMARY KEY,
    title character varying NOT NULL,
    text character varying NOT NULL,
    link character varying NOT NULL,
    link_text character varying NOT NULL,
    live boolean DEFAULT false NOT NULL,
    start_date timestamp without time zone NOT NULL,
    finish_date timestamp without time zone NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer NOT NULL
);
CREATE TABLE static_page_versions (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL,
    static_page_id bigint NOT NULL,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    title text NOT NULL,
    slug text NOT NULL,
    body text NOT NULL
);
CREATE TABLE static_pages (
    id bigserial PRIMARY KEY,
    created_at timestamp(6) without time zone NOT NULL,
    updated_at timestamp(6) without time zone NOT NULL,
    title text NOT NULL,
    slug text NOT NULL,
    body text NOT NULL
);
CREATE TABLE subnet_bans (
    id SERIAL PRIMARY KEY,
    reason character varying NOT NULL,
    note character varying,
    enabled boolean DEFAULT true NOT NULL,
    valid_until timestamp without time zone NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    banning_user_id integer NOT NULL,
    specification inet,
    generated_ban_id character varying NOT NULL
);
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name character varying NOT NULL,
    slug character varying NOT NULL,
    description character varying DEFAULT ''::character varying,
    short_description character varying DEFAULT ''::character varying,
    namespace character varying,
    name_in_namespace character varying,
    images_count integer DEFAULT 0 NOT NULL,
    image character varying,
    image_format character varying,
    image_mime_type character varying,
    aliased_tag_id integer,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    category character varying,
    mod_notes character varying
);
CREATE TABLE tags_implied_tags (
    tag_id integer NOT NULL,
    implied_tag_id integer NOT NULL,
    PRIMARY KEY (tag_id, implied_tag_id)
);
CREATE TABLE topic_subscriptions (
    topic_id integer NOT NULL,
    user_id integer NOT NULL,
    PRIMARY KEY (topic_id, user_id)
);
CREATE TABLE topics (
    id SERIAL PRIMARY KEY,
    title character varying NOT NULL,
    post_count integer DEFAULT 0 NOT NULL,
    view_count integer DEFAULT 0 NOT NULL,
    sticky boolean DEFAULT false NOT NULL,
    last_replied_to_at timestamp without time zone,
    locked_at timestamp without time zone,
    deletion_reason character varying,
    lock_reason character varying,
    slug character varying NOT NULL,
    anonymous boolean DEFAULT false,
    watcher_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watcher_count integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    forum_id integer NOT NULL,
    user_id integer,
    deleted_by_id integer,
    locked_by_id integer,
    last_post_id integer,
    hidden_from_users boolean DEFAULT false NOT NULL
);
CREATE TABLE unread_notifications (
    id SERIAL PRIMARY KEY,
    notification_id integer NOT NULL,
    user_id integer NOT NULL
);
CREATE TABLE user_bans (
    id SERIAL PRIMARY KEY,
    reason character varying NOT NULL,
    note character varying,
    enabled boolean DEFAULT true NOT NULL,
    valid_until timestamp without time zone NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer NOT NULL,
    banning_user_id integer NOT NULL,
    generated_ban_id character varying NOT NULL,
    override_ip_ban boolean DEFAULT false NOT NULL
);
CREATE TABLE user_fingerprints (
    id SERIAL PRIMARY KEY,
    fingerprint character varying NOT NULL,
    uses integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL,
    updated_at timestamp without time zone DEFAULT now() NOT NULL,
    user_id integer NOT NULL
);
CREATE TABLE user_ips (
    id SERIAL PRIMARY KEY,
    ip inet NOT NULL,
    uses integer DEFAULT 0 NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL,
    updated_at timestamp without time zone DEFAULT now() NOT NULL,
    user_id integer NOT NULL
);
CREATE TABLE user_links (
    id SERIAL PRIMARY KEY,
    aasm_state character varying NOT NULL,
    uri character varying NOT NULL,
    hostname character varying,
    path character varying,
    verification_code character varying NOT NULL,
    public boolean DEFAULT true NOT NULL,
    next_check_at timestamp without time zone,
    contacted_at timestamp without time zone,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer NOT NULL,
    verified_by_user_id integer,
    contacted_by_user_id integer,
    tag_id integer
);
CREATE TABLE user_name_changes (
    id SERIAL PRIMARY KEY,
    user_id bigint NOT NULL,
    name character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL
);
CREATE TABLE user_statistics (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    day integer DEFAULT 0 NOT NULL,
    uploads integer DEFAULT 0 NOT NULL,
    votes_cast integer DEFAULT 0 NOT NULL,
    comments_posted integer DEFAULT 0 NOT NULL,
    metadata_updates integer DEFAULT 0 NOT NULL,
    images_favourited integer DEFAULT 0 NOT NULL,
    forum_posts integer DEFAULT 0 NOT NULL
);
CREATE TABLE user_tokens (
    id bigint PRIMARY KEY,
    user_id bigint NOT NULL,
    token bytea NOT NULL,
    context character varying(255) NOT NULL,
    sent_to character varying(255),
    created_at timestamp(0) without time zone NOT NULL
);
CREATE TABLE user_whitelists (
    id SERIAL PRIMARY KEY,
    reason character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer NOT NULL
);
CREATE TABLE users (
    id serial PRIMARY KEY,
    email citext DEFAULT ''::character varying NOT NULL,
    encrypted_password character varying DEFAULT ''::character varying NOT NULL,
    reset_password_token character varying,
    reset_password_sent_at timestamp without time zone,
    remember_created_at timestamp without time zone,
    sign_in_count integer DEFAULT 0 NOT NULL,
    current_sign_in_at timestamp without time zone,
    last_sign_in_at timestamp without time zone,
    current_sign_in_ip inet,
    last_sign_in_ip inet,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    deleted_at timestamp without time zone,
    authentication_token character varying NOT NULL,
    name character varying NOT NULL,
    slug character varying NOT NULL,
    role character varying DEFAULT 'user'::character varying NOT NULL,
    description character varying,
    avatar character varying,
    spoiler_type character varying DEFAULT 'static'::character varying NOT NULL,
    theme character varying DEFAULT 'default'::character varying NOT NULL,
    images_per_page integer DEFAULT 15 NOT NULL,
    show_large_thumbnails boolean DEFAULT true NOT NULL,
    show_sidebar_and_watched_images boolean DEFAULT true NOT NULL,
    fancy_tag_field_on_upload boolean DEFAULT true NOT NULL,
    fancy_tag_field_on_edit boolean DEFAULT true NOT NULL,
    fancy_tag_field_in_settings boolean DEFAULT true NOT NULL,
    autorefresh_by_default boolean DEFAULT false NOT NULL,
    anonymous_by_default boolean DEFAULT false NOT NULL,
    scale_large_images boolean DEFAULT true NOT NULL,
    comments_newest_first boolean DEFAULT true NOT NULL,
    comments_always_jump_to_last boolean DEFAULT false NOT NULL,
    comments_per_page integer DEFAULT 20 NOT NULL,
    watch_on_reply boolean DEFAULT true NOT NULL,
    watch_on_new_topic boolean DEFAULT true NOT NULL,
    watch_on_upload boolean DEFAULT true NOT NULL,
    messages_newest_first boolean DEFAULT false NOT NULL,
    serve_webm boolean DEFAULT false NOT NULL,
    no_spoilered_in_watched boolean DEFAULT false NOT NULL,
    watched_images_query_str character varying DEFAULT ''::character varying NOT NULL,
    watched_images_exclude_str character varying DEFAULT ''::character varying NOT NULL,
    forum_posts_count integer DEFAULT 0 NOT NULL,
    topic_count integer DEFAULT 0 NOT NULL,
    recent_filter_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    unread_notification_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    watched_tag_ids integer [] DEFAULT '{}'::integer [] NOT NULL,
    deleted_by_user_id integer,
    current_filter_id integer,
    failed_attempts integer,
    unlock_token character varying,
    locked_at timestamp without time zone,
    uploads_count integer DEFAULT 0 NOT NULL,
    votes_cast_count integer DEFAULT 0 NOT NULL,
    comments_posted_count integer DEFAULT 0 NOT NULL,
    metadata_updates_count integer DEFAULT 0 NOT NULL,
    images_favourited_count integer DEFAULT 0 NOT NULL,
    last_donation_at timestamp without time zone,
    scratchpad text,
    use_centered_layout boolean DEFAULT false NOT NULL,
    secondary_role character varying,
    hide_default_role boolean DEFAULT false NOT NULL,
    personal_title character varying,
    show_hidden_items boolean DEFAULT false NOT NULL,
    hide_vote_counts boolean DEFAULT false NOT NULL,
    hide_advertisements boolean DEFAULT false NOT NULL,
    encrypted_otp_secret character varying,
    encrypted_otp_secret_iv character varying,
    encrypted_otp_secret_salt character varying,
    consumed_timestep integer,
    otp_required_for_login boolean,
    otp_backup_codes character varying [],
    last_renamed_at timestamp without time zone DEFAULT '1970-01-01 00:00:00'::timestamp without time zone NOT NULL,
    forced_filter_id bigint,
    confirmed_at timestamp(0) without time zone
);
CREATE TABLE users_roles (
    user_id integer NOT NULL,
    role_id integer NOT NULL,
    PRIMARY KEY (user_id, role_id)
);
CREATE TABLE versions (
    id serial PRIMARY KEY,
    item_type character varying NOT NULL,
    item_id integer NOT NULL,
    event character varying NOT NULL,
    whodunnit character varying,
    object text,
    created_at timestamp without time zone
);
CREATE TABLE vpns (ip inet PRIMARY KEY);
CREATE TABLE badge_awards (
    id SERIAL PRIMARY KEY,
    label character varying,
    awarded_on timestamp without time zone NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer NOT NULL,
    badge_id integer NOT NULL,
    awarded_by_id integer NOT NULL,
    reason character varying,
    badge_name character varying
);
CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    body character varying NOT NULL,
    ip inet,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    anonymous boolean DEFAULT false,
    hidden_from_users boolean DEFAULT false NOT NULL,
    user_id integer,
    deleted_by_id integer,
    image_id integer,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    edit_reason character varying,
    edited_at timestamp without time zone,
    deletion_reason character varying DEFAULT ''::character varying NOT NULL,
    destroyed_content boolean DEFAULT false,
    name_at_post_time character varying
);
CREATE TABLE source_changes (
    id SERIAL PRIMARY KEY,
    ip inet NOT NULL,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    new_value character varying,
    initial boolean DEFAULT false NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer,
    image_id integer NOT NULL
);
CREATE TABLE tag_changes (
    id SERIAL PRIMARY KEY,
    ip inet,
    fingerprint character varying,
    user_agent character varying DEFAULT ''::character varying,
    referrer character varying DEFAULT ''::character varying,
    added boolean NOT NULL,
    tag_name_cache character varying DEFAULT ''::character varying NOT NULL,
    created_at timestamp without time zone NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    user_id integer,
    tag_id integer,
    image_id integer NOT NULL
);
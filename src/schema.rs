// @generated automatically by Diesel CLI.

diesel::table! {
    #[allow(non_snake_case)]
    RAMP_Locations (RARID) {
        RARID -> Integer,
        name -> Nullable<Text>,
        abbr -> Nullable<Text>,
        address -> Nullable<Text>,
        city -> Nullable<Text>,
        prov -> Nullable<Text>,
        #[max_length = 64]
        province_name -> Nullable<Varchar>,
        pcode -> Nullable<Text>,
        country -> Nullable<Text>,
        phone -> Nullable<Text>,
        liveBarnId -> Nullable<Text>,
        #[max_length = 128]
        loc -> Nullable<Varchar>,
        location_id -> Integer,
        #[max_length = 128]
        match_type -> Nullable<Varchar>,
        surface_id -> Nullable<Integer>,
    }
}

diesel::table! {
    events (id) {
        id -> Unsigned<Bigint>,
        #[max_length = 64]
        site -> Varchar,
        #[max_length = 64]
        source_type -> Nullable<Varchar>,
        datetime -> Datetime,
        #[max_length = 128]
        home_team -> Varchar,
        #[max_length = 128]
        oid_home -> Nullable<Varchar>,
        #[max_length = 128]
        guest_team -> Varchar,
        #[max_length = 128]
        oid_guest -> Nullable<Varchar>,
        #[max_length = 128]
        location -> Nullable<Varchar>,
        #[max_length = 128]
        division -> Nullable<Varchar>,
        location_id -> Nullable<Integer>,
        surface_id -> Integer,
        date_created -> Timestamp,
    }
}

diesel::table! {
    feed_modes (id) {
        id -> Integer,
        #[max_length = 64]
        feed_mode -> Varchar,
    }
}

diesel::table! {
    gamesheet_schedules (id) {
        id -> Unsigned<Bigint>,
        season_id -> Unsigned<Integer>,
        game_data -> Json,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    gamesheet_seasons (id) {
        id -> Unsigned<Integer>,
        #[max_length = 200]
        title -> Nullable<Varchar>,
        #[max_length = 100]
        site -> Varchar,
        league_id -> Nullable<Unsigned<Integer>>,
        is_active -> Nullable<Tinyint>,
        start_date -> Nullable<Date>,
        end_date -> Nullable<Date>,
    }
}

diesel::table! {
    gthl_mappings (location) {
        #[max_length = 64]
        location -> Varchar,
        surface_id -> Integer,
    }
}

diesel::table! {
    locations (id) {
        id -> Integer,
        address1 -> Nullable<Text>,
        address2 -> Nullable<Text>,
        #[max_length = 32]
        city -> Nullable<Varchar>,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 128]
        uuid -> Nullable<Varchar>,
        #[max_length = 32]
        recording_hours_local -> Nullable<Varchar>,
        #[max_length = 32]
        postal_code -> Nullable<Varchar>,
        all_sheets_count -> Nullable<Integer>,
        longitude -> Nullable<Float>,
        latitude -> Nullable<Float>,
        logo_url -> Nullable<Text>,
        province_id -> Nullable<Integer>,
        #[max_length = 11]
        venue_status -> Nullable<Varchar>,
        #[max_length = 32]
        zone -> Nullable<Varchar>,
        total_surfaces -> Nullable<Integer>,
        deleted_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    mhl_mappings (location) {
        #[max_length = 64]
        location -> Varchar,
        surface_id -> Integer,
    }
}

diesel::table! {
    nyhl_mappings (location) {
        #[max_length = 64]
        location -> Varchar,
        surface_id -> Integer,
    }
}

diesel::table! {
    ohf_teams (team_number) {
        #[max_length = 128]
        team_number -> Varchar,
        team_name -> Nullable<Text>,
        team_organization -> Nullable<Text>,
        team_organization_path -> Nullable<Text>,
        team_gender_identity -> Nullable<Text>,
        division_name -> Nullable<Text>,
        registrations_class_name -> Nullable<Text>,
        category_name -> Nullable<Text>,
    }
}

diesel::table! {
    provinces (id) {
        id -> Integer,
        #[max_length = 32]
        province_name -> Varchar,
        #[max_length = 32]
        country -> Varchar,
    }
}

diesel::table! {
    renditions (id) {
        id -> Integer,
        surface_id -> Integer,
        #[max_length = 32]
        name -> Varchar,
        width -> Integer,
        height -> Integer,
        #[max_length = 16]
        ratio -> Varchar,
        bitrate -> Unsigned<Bigint>,
    }
}

diesel::table! {
    schema_migrations (version) {
        version -> Bigint,
        dirty -> Bool,
    }
}

diesel::table! {
    session (session_key) {
        #[max_length = 64]
        session_key -> Char,
        session_data -> Nullable<Blob>,
        session_expiry -> Unsigned<Integer>,
    }
}

diesel::table! {
    sites_config (id) {
        id -> Integer,
        #[max_length = 100]
        site_name -> Varchar,
        #[max_length = 200]
        display_name -> Nullable<Varchar>,
        #[max_length = 500]
        base_url -> Varchar,
        #[max_length = 100]
        home_team -> Nullable<Varchar>,
        parser_config -> Nullable<Json>,
        enabled -> Nullable<Bool>,
        last_scraped_at -> Nullable<Timestamp>,
        scrape_frequency_hours -> Nullable<Integer>,
        notes -> Nullable<Text>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        #[max_length = 64]
        parser_type -> Varchar,
    }
}

diesel::table! {
    sites_locations (site, location) {
        #[max_length = 64]
        site -> Varchar,
        #[max_length = 128]
        location -> Varchar,
        location_id -> Nullable<Integer>,
        #[max_length = 64]
        loc -> Nullable<Varchar>,
        #[max_length = 64]
        surface -> Nullable<Varchar>,
        #[max_length = 128]
        address -> Nullable<Varchar>,
        #[max_length = 32]
        match_type -> Nullable<Varchar>,
        surface_id -> Integer,
    }
}

diesel::table! {
    surface_feed_modes (surface_id, feed_mode_id) {
        surface_id -> Integer,
        feed_mode_id -> Integer,
    }
}

diesel::table! {
    surfaces (id) {
        id -> Integer,
        location_id -> Integer,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 128]
        uuid -> Varchar,
        orderIndex -> Integer,
        venue_id -> Integer,
        closed_from -> Unsigned<Bigint>,
        coming_soon -> Bool,
        online -> Bool,
        #[max_length = 32]
        status -> Varchar,
        #[max_length = 32]
        sports -> Varchar,
        first_media_date -> Unsigned<Bigint>,
        deleted_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    todb_surfaces (id) {
        id -> Integer,
        #[max_length = 256]
        fullname -> Nullable<Varchar>,
        #[max_length = 256]
        fullshortname -> Nullable<Varchar>,
        #[max_length = 256]
        street -> Nullable<Varchar>,
        #[max_length = 256]
        city -> Nullable<Varchar>,
        #[max_length = 45]
        province -> Nullable<Varchar>,
    }
}

diesel::table! {
    users (username) {
        #[max_length = 16]
        username -> Varchar,
        #[max_length = 64]
        password -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(locations -> provinces (province_id));
diesel::joinable!(renditions -> surfaces (surface_id));
diesel::joinable!(surface_feed_modes -> feed_modes (feed_mode_id));
diesel::joinable!(surface_feed_modes -> surfaces (surface_id));
diesel::joinable!(surfaces -> locations (location_id));

diesel::allow_tables_to_appear_in_same_query!(
    RAMP_Locations,
    events,
    feed_modes,
    gamesheet_schedules,
    gamesheet_seasons,
    gthl_mappings,
    locations,
    mhl_mappings,
    nyhl_mappings,
    ohf_teams,
    provinces,
    renditions,
    schema_migrations,
    session,
    sites_config,
    sites_locations,
    surface_feed_modes,
    surfaces,
    todb_surfaces,
    users,
);

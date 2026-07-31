-- Complete the Steam profile and enforce that every user keeps a login method.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS steam_country_code varchar(2);

ALTER TABLE users
    ADD CONSTRAINT users_authentication_method_check
        CHECK (password_hash IS NOT NULL OR steam_id IS NOT NULL);

ALTER TABLE users
    ADD CONSTRAINT users_steam_country_code_check
        CHECK (
            steam_country_code IS NULL
            OR steam_country_code ~ '^[A-Z]{2}$'
        );

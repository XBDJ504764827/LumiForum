ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_steam_country_code_check,
    DROP CONSTRAINT IF EXISTS users_authentication_method_check,
    DROP COLUMN IF EXISTS steam_country_code;

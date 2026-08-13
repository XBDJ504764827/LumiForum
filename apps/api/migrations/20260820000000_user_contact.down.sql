ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_contact_check;

ALTER TABLE users
    DROP COLUMN IF EXISTS contact;

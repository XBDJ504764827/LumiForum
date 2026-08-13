ALTER TABLE users
    ADD COLUMN contact varchar(128);

ALTER TABLE users
    ADD CONSTRAINT users_contact_check
        CHECK (
            contact IS NULL
            OR (contact = btrim(contact) AND char_length(contact) BETWEEN 1 AND 128)
        );

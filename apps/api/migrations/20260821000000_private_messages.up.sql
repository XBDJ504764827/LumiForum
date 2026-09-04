-- Private messaging (DM): a conversation per participant pair plus messages.
-- A conversation is identified by the unordered pair of participants (stored
-- sorted in user1_id/user2_id so both directions share one row). Reads are
-- tracked per conversation with a single last_read_at per side.

CREATE TABLE conversations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user1_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user2_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Read markers live on the conversation row so listing a conversation and
    -- its unread state is a single-table query.
    user1_last_read_at timestamptz,
    user2_last_read_at timestamptz,
    -- Denormalized counters keep the inbox list cheap; verified on read.
    user1_unread int NOT NULL DEFAULT 0,
    user2_unread int NOT NULL DEFAULT 0,
    last_message_at timestamptz NOT NULL DEFAULT now(),
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT conversations_pair_check CHECK (user1_id <> user2_id),
    -- Enforce the sorted-pair invariant in the database itself.
    CONSTRAINT conversations_pair_order_check CHECK (user1_id < user2_id),
    CONSTRAINT conversations_pair_uidx UNIQUE (user1_id, user2_id)
);

CREATE INDEX conversations_user1_idx
    ON conversations (user1_id, last_message_at DESC);

CREATE INDEX conversations_user2_idx
    ON conversations (user2_id, last_message_at DESC);

CREATE TABLE messages (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id uuid NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    edited_at timestamptz,
    -- Soft delete per message; deleted text is replaced by a placeholder.
    deleted_at timestamptz,
    CONSTRAINT messages_content_check
        CHECK (char_length(content) BETWEEN 1 AND 2000)
);

-- sender_id must be one of the two participants; enforced in the service layer
-- (Postgres CHECK constraints cannot reference other rows).

CREATE INDEX messages_conversation_created_idx
    ON messages (conversation_id, created_at DESC, id DESC);

CREATE INDEX messages_sender_idx ON messages (sender_id, created_at DESC);
-- Phase 14.1: poll authors may prevent voters from cancelling cast votes.
ALTER TABLE polls
    ADD COLUMN IF NOT EXISTS allow_cancel boolean NOT NULL DEFAULT true;

-- Add migration script here
CREATE TABLE "syncs" (
    guild_id TEXT NOT NULL,
    server_name TEXT,
    meetup_group TEXT NOT NULL,
    number_of_events INTEGER DEFAULT 2 NOT NULL,
    voice_channel_id TEXT NOT NULL
)
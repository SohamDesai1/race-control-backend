-- Fantasy Constructors table
CREATE TABLE IF NOT EXISTS "fantasy_constructors" (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    salary INTEGER NOT NULL DEFAULT 15000000,
    year INTEGER NOT NULL DEFAULT 2026
);
-- Fantasy Drivers table
CREATE TABLE IF NOT EXISTS "fantasy_drivers" (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    driver_id INTEGER NOT NULL,
    code VARCHAR(3) NOT NULL,
    team_id INTEGER REFERENCES "fantasy_constructors"(id),
    salary INTEGER NOT NULL DEFAULT 15000000,
    year INTEGER NOT NULL DEFAULT 2026
);
-- Add foreign key to fantasy_drivers after constructors exist
ALTER TABLE "fantasy_drivers"
ADD CONSTRAINT fk_driver_constructor FOREIGN KEY (team_id) REFERENCES "fantasy_constructors"(id);
-- Fantasy Contests table
CREATE TABLE IF NOT EXISTS "fantasy_contests" (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    creator_id INTEGER REFERENCES "Users"(id),
    contest_type VARCHAR(10) NOT NULL CHECK (contest_type IN ('gp', 'season')),
    gp_id INTEGER REFERENCES "Races"(id),
    invite_code VARCHAR(6) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
-- Fantasy Contest Participants table
CREATE TABLE IF NOT EXISTS "fantasy_contest_participants" (
    id SERIAL PRIMARY KEY,
    contest_id INTEGER REFERENCES "fantasy_contests"(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES "Users"(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(contest_id, user_id)
);
-- Fantasy Teams table
CREATE TABLE IF NOT EXISTS "fantasy_teams" (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES "Users"(id) ON DELETE CASCADE,
    contest_id INTEGER REFERENCES "fantasy_contests"(id) ON DELETE CASCADE,
    gp_id INTEGER REFERENCES "Races"(id),
    driver_1_id INTEGER REFERENCES "fantasy_drivers"(id),
    driver_2_id INTEGER REFERENCES "fantasy_drivers"(id),
    constructor_id INTEGER REFERENCES "fantasy_constructors"(id),
    booster_driver_id INTEGER REFERENCES "fantasy_drivers"(id),
    budget_used INTEGER NOT NULL DEFAULT 0,
    is_locked BOOLEAN NOT NULL DEFAULT FALSE,
    total_points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_fantasy_drivers_team ON "fantasy_drivers"(team_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_drivers_code ON "fantasy_drivers"(code);
CREATE INDEX IF NOT EXISTS idx_fantasy_drivers_year ON "fantasy_drivers"(year);
CREATE INDEX IF NOT EXISTS idx_fantasy_constructors_year ON "fantasy_constructors"(year);
CREATE INDEX IF NOT EXISTS idx_fantasy_contests_invite_code ON "fantasy_contests"(invite_code);
CREATE INDEX IF NOT EXISTS idx_fantasy_contests_creator ON "fantasy_contests"(creator_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_contest_participants_contest ON "fantasy_contest_participants"(contest_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_contest_participants_user ON "fantasy_contest_participants"(user_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_teams_user_gp ON "fantasy_teams"(user_id, gp_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_teams_contest_gp ON "fantasy_teams"(contest_id, gp_id);
CREATE INDEX IF NOT EXISTS idx_fantasy_teams_gp_points ON "fantasy_teams"(gp_id, total_points DESC);
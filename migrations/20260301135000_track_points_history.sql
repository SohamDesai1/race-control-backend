CREATE TABLE IF NOT EXISTS "DriverPointsHistory" (
    id SERIAL PRIMARY KEY,
    driver_number VARCHAR NOT NULL,
    session_key INTEGER NOT NULL,
    meeting_key INTEGER,
    season VARCHAR NOT NULL,
    round VARCHAR NOT NULL,
    race_id INTEGER REFERENCES "Races"(id),
    points_start FLOAT DEFAULT 0,
    points_current FLOAT DEFAULT 0,
    position INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(driver_number, session_key)
);

CREATE TABLE IF NOT EXISTS "ConstructorPointsHistory" (
    id SERIAL PRIMARY KEY,
    constructor_id VARCHAR NOT NULL,
    constructor_name VARCHAR NOT NULL,
    session_key INTEGER NOT NULL,
    meeting_key INTEGER,
    season VARCHAR NOT NULL,
    round VARCHAR NOT NULL,
    race_id INTEGER REFERENCES "Races"(id),
    points_start FLOAT DEFAULT 0,
    points_current FLOAT DEFAULT 0,
    position INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(constructor_id, session_key)
);

CREATE INDEX IF NOT EXISTS idx_driver_championship_season 
ON "DriverPointsHistory"(season, driver_number, round);

CREATE INDEX IF NOT EXISTS idx_constructor_points_season 
ON "ConstructorPointsHistory"(season, constructor_id, round);
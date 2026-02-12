-- Create Users table
CREATE TABLE IF NOT EXISTS "Users" (
    id SERIAL PRIMARY KEY,
    name VARCHAR,
    username VARCHAR UNIQUE,
    email VARCHAR UNIQUE NOT NULL,
    dob VARCHAR,
    gender VARCHAR,
    hashed_password VARCHAR,
    auth_provider VARCHAR,
    is_profile_complete BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create Circuits table
CREATE TABLE IF NOT EXISTS "Circuits" (
    "circuitId" VARCHAR PRIMARY KEY,
    "circuitName" VARCHAR,
    location VARCHAR,
    country VARCHAR,
    lat VARCHAR,
    long VARCHAR,
    locality VARCHAR
);

-- Create Races table
CREATE TABLE IF NOT EXISTS "Races" (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    season VARCHAR,
    round VARCHAR,
    date DATE,
    time TIME,
    "raceName" VARCHAR,
    "circuitId" VARCHAR REFERENCES "Circuits"("circuitId")
);

-- Create Sessions table
CREATE TABLE IF NOT EXISTS "Sessions" (
    id SERIAL PRIMARY KEY,
    "raceId" INTEGER REFERENCES "Races"(id),
    "sessionType" VARCHAR,
    date DATE,
    time TIME,
    "session_key" INTEGER,
    "meeting_key" INTEGER
);

-- Create NewsCache table
CREATE TABLE IF NOT EXISTS "NewsCache" (
    id SERIAL PRIMARY KEY,
    source VARCHAR,
    title VARCHAR,
    description TEXT,
    url VARCHAR,
    image VARCHAR,
    published_at VARCHAR,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

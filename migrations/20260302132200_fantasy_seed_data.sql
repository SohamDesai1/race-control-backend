-- Add year column to support multiple seasons (if not already added)
ALTER TABLE "fantasy_drivers" ADD COLUMN IF NOT EXISTS year INTEGER NOT NULL DEFAULT 2026;
ALTER TABLE "fantasy_constructors" ADD COLUMN IF NOT EXISTS year INTEGER NOT NULL DEFAULT 2026;

-- Seed Fantasy Constructors (2026 F1 Teams) - matching frontend
INSERT INTO "fantasy_constructors" (name, salary, year) VALUES
('Red Bull', 30000000, 2026),
('Ferrari', 25000000, 2026),
('Mercedes', 28000000, 2026),
('McLaren', 35000000, 2026),
('Aston Martin', 16000000, 2026),
('Alpine F1 Team', 12000000, 2026),
('Williams', 20000000, 2026),
('RB F1 Team', 16000000, 2026),
('Audi', 15000000, 2026),
('Haas F1 Team', 13000000, 2026),
('Cadillac', 15000000, 2026)
ON CONFLICT DO NOTHING;

-- Seed Fantasy Drivers (2026 F1 Drivers) - matching frontend
INSERT INTO "fantasy_drivers" (name, driver_id, code, salary, year) VALUES
('Max Verstappen', 3, 'VER', 38000000, 2026),
('Isack Hadjar', 6, 'HAD', 14000000, 2026),
('Charles Leclerc', 16, 'LEC', 28000000, 2026),
('Lewis Hamilton', 44, 'HAM', 25000000, 2026),
('George Russell', 63, 'RUS', 27000000, 2026),
('Andrea Kimi Antonelli', 12, 'ANT', 18000000, 2026),
('Lando Norris', 1, 'NOR', 32000000, 2026),
('Oscar Piastri', 81, 'PIA', 30000000, 2026),
('Fernando Alonso', 14, 'ALO', 16000000, 2026),
('Lance Stroll', 18, 'STR', 10000000, 2026),
('Pierre Gasly', 10, 'GAS', 12000000, 2026),
('Franco Colapinto', 43, 'COL', 5000000, 2026),
('Alexander Albon', 23, 'ALB', 14000000, 2026),
('Carlos Sainz', 55, 'SAI', 18000000, 2026),
('Liam Lawson', 30, 'LAW', 13000000, 2026),
('Yuki Tsunoda', 22, 'TSU', 9000000, 2026),
('Nico Hülkenberg', 27, 'HUL', 13000000, 2026),
('Gabriel Bortoleto', 5, 'BOR', 10000000, 2026),
('Oliver Bearman', 87, 'BEA', 9000000, 2026),
('Esteban Ocon', 31, 'OCO', 8000000, 2026),
('Sergio Perez', 11, 'PER', 8000000, 2026),
('Valtteri Bottas', 77, 'BOT', 6000000, 2026)
ON CONFLICT DO NOTHING;

-- Update team_id for drivers based on constructor names
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Red Bull' AND year = 2026) WHERE name = 'Max Verstappen' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Red Bull' AND year = 2026) WHERE name = 'Isack Hadjar' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Ferrari' AND year = 2026) WHERE name = 'Charles Leclerc' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Ferrari' AND year = 2026) WHERE name = 'Lewis Hamilton' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Mercedes' AND year = 2026) WHERE name = 'George Russell' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Mercedes' AND year = 2026) WHERE name = 'Andrea Kimi Antonelli' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'McLaren' AND year = 2026) WHERE name = 'Lando Norris' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'McLaren' AND year = 2026) WHERE name = 'Oscar Piastri' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Aston Martin' AND year = 2026) WHERE name = 'Fernando Alonso' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Aston Martin' AND year = 2026) WHERE name = 'Lance Stroll' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Alpine F1 Team' AND year = 2026) WHERE name = 'Pierre Gasly' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Alpine F1 Team' AND year = 2026) WHERE name = 'Franco Colapinto' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Williams' AND year = 2026) WHERE name = 'Alexander Albon' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Williams' AND year = 2026) WHERE name = 'Carlos Sainz' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'RB F1 Team' AND year = 2026) WHERE name = 'Liam Lawson' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'RB F1 Team' AND year = 2026) WHERE name = 'Yuki Tsunoda' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Audi' AND year = 2026) WHERE name = 'Nico Hülkenberg' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Audi' AND year = 2026) WHERE name = 'Gabriel Bortoleto' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Haas F1 Team' AND year = 2026) WHERE name = 'Oliver Bearman' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Haas F1 Team' AND year = 2026) WHERE name = 'Esteban Ocon' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Cadillac' AND year = 2026) WHERE name = 'Sergio Perez' AND year = 2026;
UPDATE "fantasy_drivers" SET team_id = (SELECT id FROM "fantasy_constructors" WHERE name = 'Cadillac' AND year = 2026) WHERE name = 'Valtteri Bottas' AND year = 2026;
# Fantasy F1 v2 - Planning Document

## Overview
Fantasy F1 v2 adds a fantasy racing aspect where users can build fantasy teams for each Grand Prix, compete in private contests, and climb leaderboards based on driver/constructor performance.

## Features

### Team Composition
- **Budget**: $100M total
- **Drivers**: 2 drivers per team
- **Constructor**: 1 constructor
- **Booster**: 1 driver can be selected for 2x points (once per GP)

### Contests
- **Types**: Single GP or Full Season
- **Join method**: Unique invite code (6-character alphanumeric)
- **Participants**: Unlimited
- **Visibility**: Private (by invite code only)

### Scoring System (Points from Team Performance)
Points are awarded based on the actual race results of your selected drivers and constructor:

| Your Driver Finishes | Points |
|---------------------|--------|
| 1st place | 25 pts |
| 2nd place | 18 pts |
| 3rd place | 15 pts |
| 4th place | 12 pts |
| 5th place | 10 pts |
| 6th place | 8 pts |
| 7th place | 6 pts |
| 8th place | 4 pts |
| 9th place | 2 pts |
| 10th place | 1 pt |

| Your Constructor Finishes | Points |
|---------------------------|--------|
| 1st place | 25 pts |
| 2nd place | 18 pts |
| 3rd place | 15 pts |
| 4th place | 12 pts |
| 5th place | 10 pts |
| 6th place | 8 pts |
| 7th place | 6 pts |
| 8th place | 4 pts |
| 9th place | 2 pts |
| 10th place | 1 pt |

| Bonus | Points |
|-------|--------|
| Fastest lap (any driver in team) | 5 pts |
| Booster driver points | 2x multiplier |

Performance-Based Pricing
After each GP, salaries adjust based on results:
| Finishing Position | Price Change |
|-------------------|--------------|
| 1st place | +$5M |
| 2nd place | +$3M |
| 3rd place | +$2M |
| 4th-6th | +$1M |
| 7th-10th | $0 (no change) |
| 11th+ | -$1M |
| DNF | -$2M |
Bonuses:
- Win +$2M extra
- Fastest lap +$1M extra
Price Limits: $5M (min) to $50M (max)
Example
Verstappen wins → salary goes from $30M → $37M (+$5M win + $2M bonus)
Driver finishes 15th → salary $20M → $19M (-$1M)
---

Constructor Performance-Based Pricing
After each GP, constructor prices adjust based on combined points from both drivers:
| Combined Driver Points | Price Change |
|----------------------|--------------|
| 1st + 1st (dominant) | +$8M |
| 1st + 2nd | +$6M |
| 1st + 3rd | +$5M |
| Top 3 total | +$4M |
| Top 6 total | +$2M |
| Top 10 total | +$1M |
| No points | -$2M |
| Both DNF | -$3M |
Bonuses:
- Win (1st + 2nd) +$3M extra
- Both drivers in points +$1M extra
Example
Red Bull: Verstappen 1st (+25) + Perez 4th (+12) = 37 combined → +$6M
Haas: Both drivers outside top 10 = -$2M
Price Limits: $5M (min) to $50M (max)
---
Edge Cases for Constructors
1. Team completely new (new team joins)
   - Default salary: $15M
2. Team name change (e.g., AlphaTauri → RB)
   - Keep previous salary under new name
3. Driver changes mid-season
   - Constructor salary persists, only driver-specific prices change

   
### Lock Logic
- Team locks when qualifying session starts (check OpenF1 API for qualifying session)
- Booster can be changed until lock

---

## Database Schema

### fantasy_drivers
Purpose: Master list of all F1 drivers with their fantasy salaries

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| name | VARCHAR(255) | Driver full name |
| code | VARCHAR(3) | Driver code (e.g., VER, LEC) |
| team_id | INTEGER | FK to fantasy_constructors |
| salary | INTEGER | Fantasy price in millions |

### fantasy_constructors
Purpose: Master list of F1 teams/constructors with fantasy prices

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| name | VARCHAR(255) | Constructor name |
| salary | INTEGER | Fantasy price in millions |

### fantasy_contests
Purpose: Private contests users create to compete with friends

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| name | VARCHAR(255) | Contest name |
| creator_id | INTEGER | FK to users (creator) |
| contest_type | VARCHAR(10) | 'gp' or 'season' |
| gp_id | INTEGER | FK to races (nullable for season) |
| invite_code | VARCHAR(6) | Unique invite code |
| created_at | TIMESTAMP | Creation timestamp |

### fantasy_contest_participants
Purpose: Tracks which users have joined which contests

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| contest_id | INTEGER | FK to fantasy_contests |
| user_id | INTEGER | FK to users |
| joined_at | TIMESTAMP | Join timestamp |

### fantasy_teams
Purpose: User's fantasy team for a specific GP within a contest

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| user_id | INTEGER | FK to users |
| contest_id | INTEGER | FK to fantasy_contests (nullable for global) |
| gp_id | INTEGER | FK to races |
| driver_1_id | INTEGER | FK to fantasy_drivers |
| driver_2_id | INTEGER | FK to fantasy_drivers |
| constructor_id | INTEGER | FK to fantasy_constructors |
| booster_driver_id | INTEGER | FK to fantasy_drivers (nullable) |
| budget_used | INTEGER | Total salary used |
| is_locked | BOOLEAN | Locked after qualifying |
| total_points | INTEGER | Accumulated points |
| created_at | TIMESTAMP | Creation timestamp |
| updated_at | TIMESTAMP | Last update |

## How They Work Together
User creates contest (fantasy_contests)
    ↓
User joins contest (fantasy_contest_participants)
    ↓
User creates team for a GP (fantasy_teams)
    ├── picks 2 drivers
    ├── picks 1 constructor
    └── sets booster (once per GP)
    ↓
After race: calculate points based on:
    ├── Driver finishing positions
    ├── Constructor finishing positions
    ├── Fastest lap bonus
    └── Booster multiplier
    ↓
Leaderboards show rankings (global & per-contest)
---
## Key Constraints
- Budget: Total driver + constructor salary ≤ $100M
- Booster: Can only be used once per GP per user
- Lock: Team freezes when qualifying session starts
- Invite Code: Unique 6-character alphanumeric code per contest

---

## API Endpoints

### Drivers & Constructors
| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | `/fantasy/drivers` | List all drivers with salaries | Yes |
| GET | `/fantasy/constructors` | List all constructors with salaries | Yes |

### Contests
| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| POST | `/fantasy/contests` | Create a new contest | Yes |
| GET | `/fantasy/contests` | List user's contests | Yes |
| GET | `/fantasy/contests/{invite_code}` | Get contest by invite code | Yes |
| POST | `/fantasy/contests/{invite_code}/join` | Join contest with code | Yes |
| DELETE | `/fantasy/contests/{id}/leave` | Leave a contest | Yes |
| GET | `/fantasy/contests/{id}` | Get contest details + participants | Yes |

### Team Management
| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | `/fantasy/gp/{gp_id}` | Get GP info + available picks | Yes |
| GET | `/fantasy/gp/{gp_id}/team` | Get user's team for GP | Yes |
| POST | `/fantasy/gp/{gp_id}/team` | Create/update team | Yes |
| POST | `/fantasy/gp/{gp_id}/booster` | Set/update booster driver | Yes |
| DELETE | `/fantasy/gp/{gp_id}/team` | Delete team for GP | Yes |

### Leaderboards
| Method | Endpoint | Description | Auth |
|--------|----------|-------------|------|
| GET | `/fantasy/leaderboard` | Global season leaderboard | Yes |
| GET | `/fantasy/gp/{gp_id}/leaderboard` | Global GP leaderboard | Yes |
| GET | `/fantasy/contests/{id}/leaderboard` | Contest leaderboard | Yes |

---

## Implementation Order

### Phase 1: Database & Models
1. Create database migrations for all fantasy tables
2. Add fantasy_drivers and fantasy_constructors seed data with salaries
3. Create Rust models for all tables

### Phase 2: Core Routes & Handlers
4. Build fantasy_drivers and fantasy_constructors endpoints
5. Build contest creation and management endpoints
6. Build team creation and update endpoints

### Phase 3: Game Logic
7. Implement budget validation
8. Implement team lock logic (check qualifying session)
9. Implement booster functionality (once per GP per user)

### Phase 4: Scoring & Leaderboards
10. Create scoring calculator (runs after race completion)
11. Build global leaderboard endpoints
12. Build contest-specific leaderboard endpoints

### Phase 5: Polish
13. Add input validation
14. Add error handling
15. Test all endpoints

---

## Scoring Calculation Plan

### Data Sources
| Data | Source |
|------|--------|
| Driver finishing positions | OpenF1: `session_result?session_key={session_key}` |
| Constructor finishing positions | Ergast API: `/f1/{year}/{round}/constructorstandings/` |
| Fastest lap driver | OpenF1: Query laps and find minimum lap_duration |

### Automatic Scoring (Background Worker)

The system runs a background worker that:
1. **Polls OpenF1** for race sessions every 5 minutes
2. **Detects race completion**: When session_type = "Race" and session_status = "Finished"
3. **Triggers scoring calculation** for that GP

### Scoring Algorithm

```
For each team in the GP:
  total_points = 0
  
  // Driver 1 points
  driver1_position = get_driver_position(driver1_id)
  total_points += get_finishing_points(driver1_position)
  
  // Driver 2 points
  driver2_position = get_driver_position(driver2_id)
  total_points += get_finishing_points(driver2_position)
  
  // Constructor points
  constructor_position = get_constructor_position(constructor_id)
  total_points += get_finishing_points(constructor_position)
  
  // Fastest lap bonus
  if fastest_lap_driver in [driver1_id, driver2_id]:
    total_points += 5
  
  // Apply booster (2x for the selected driver only)
  if booster_driver_id == driver1_id:
    total_points += get_finishing_points(driver1_position)  // Add again (2x)
  if booster_driver_id == driver2_id:
    total_points += get_finishing_points(driver2_position)  // Add again (2x)
  
  // Update team in database
  UPDATE fantasy_teams SET total_points = total_points WHERE id = team.id
```

### Winner Determination

| Contest Type | Winner Logic |
|--------------|--------------|
| Single GP | Highest `total_points` for that GP |
| Season | Sum of all GP `total_points` across the season |

### Background Job Details

- **Polling interval**: Every 5 minutes
- **GP tracking**: Store last processed session_key per gp_id
- **Idempotency**: Skip if points already calculated (check `is_locked` = true + points > 0)

The plan mentions a background worker - which runs inside the Rust application using Tokio. Here's how it works:
Background Worker (Internal)
// In your main.rs or app startup
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 min
    
    loop {
        interval.tick().await;
        check_and_calculate_scores().await;
    }
});
This runs continuously as part of your app - no external cron needed.

Option 1: Smart Polling
Only poll frequently when a race is actually happening:
Check OpenF1 sessions API
    ↓
Is there an active race session?
    ├── YES → Poll every 5 min for results
    └── NO → Poll once per hour (just to detect race start)
Option 2: Time-Based (Race Weekends Only)
Only run during known F1 weekends:
- Know the schedule (from Ergast or hardcoded)
- Only enable polling during race weekend (Fri-Sun)
Option 3: Hybrid
- Before race weekend: No polling
- During race weekend: Poll every 5 min
- Manual override: Admin can force calculate anytime
---
Recommendation
Option 2 (Time-Based) is most efficient:
1. Fetch race schedule from Ergast API at startup
2. Only run worker during race weekend dates
3. Check every 5 min during Friday-Sunday
This way the worker is essentially "off" most of the time and only activates when there's actually a race.


### API for Manual Trigger (Admin)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/admin/fantasy/calculate/{gp_id}` | Manually trigger scoring for a GP |
| GET | `/admin/fantasy/status` | Check scoring job status |

---

## Acceptance Criteria

1. Users can view all drivers and constructors with their fantasy salaries
2. Users can create a private contest with an invite code
3. Users can join a contest using an invite code
4. Users can create a fantasy team for any GP within budget
5. Users can set a booster driver (once per GP)
6. Teams lock when qualifying starts
7. Points are calculated correctly after race
8. Leaderboards show correct rankings (global and contest)
9. All endpoints require authentication
10. Budget validation prevents overspending

opencode -s ses_3a2cbc191ffeszNxrTL3EoXNzF
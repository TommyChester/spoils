# Spoils

A monorepo containing a Rust backend API and an Expo (React Native) mobile frontend.

## Project Structure

```
spoils/
├── backend/          # Rust API with Actix-web and Diesel
│   ├── src/
│   ├── migrations/   # Database migrations
│   ├── Cargo.toml
│   └── .env.example
├── mobile/           # Expo mobile app (TypeScript)
│   ├── api/          # API client
│   ├── App.tsx
│   └── package.json
└── README.md
```

## Tech Stack

**Backend:**
- Rust with Actix-web (web framework)
- Diesel (ORM for PostgreSQL)
- Serde (JSON serialization)

**Frontend:**
- React Native with Expo
- TypeScript
- Expo SDK for native features

## Getting Started

### Prerequisites

- Rust and Cargo
- Node.js (v20+) and npm
- PostgreSQL (for database)
- Diesel CLI: `cargo install diesel_cli --no-default-features --features postgres`

### Backend Setup

1. Navigate to the backend directory:
   ```bash
   cd backend
   ```

2. Copy the environment file and configure your database:
   ```bash
   cp .env.example .env
   # Edit .env with your PostgreSQL credentials
   ```

3. Set up the database:
   ```bash
   diesel setup
   ```

4. Run the backend:
   ```bash
   cargo run
   ```

   The API will start on `http://localhost:8080`

### Mobile Setup

1. Navigate to the mobile directory:
   ```bash
   cd mobile
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Start the Expo development server:
   ```bash
   npm start
   ```

4. Scan the QR code with Expo Go app (iOS/Android) or press `i` for iOS simulator or `a` for Android emulator.

## API Endpoints

- `GET /health` - Health check endpoint
- `GET /api/hello` - Test endpoint

## Development

### Running Both Services

Terminal 1 (Backend):
```bash
cd backend && cargo run
```

Terminal 2 (Mobile):
```bash
cd mobile && npm start
```

### Database Migrations

Create a new migration:
```bash
cd backend
diesel migration generate <migration_name>
```

Run migrations:
```bash
diesel migration run
```

Revert last migration:
```bash
diesel migration revert
```

## Deployment

**Backend:**
- Deployed to Heroku at: https://spoils-backend-82d4a06b8d67.herokuapp.com
- PostgreSQL database: ✓ Configured
- To redeploy: `git subtree push --prefix backend heroku main`

**Mobile:**
- Use Expo EAS Build for production builds
- First time setup: `cd mobile && npx eas-cli login && npx eas-cli build:configure`
- Create a build: `npx eas-cli build --platform ios` or `--platform android`
- The app is configured to use the production API URL by default

## License

MIT

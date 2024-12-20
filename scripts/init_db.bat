set DATABASE_URL=postgres://postgres:password@localhost:5432/postgres
sqlx database create
sqlx migrate run

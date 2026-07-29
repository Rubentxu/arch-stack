package store

import "database/sql"

type SQLite struct {
	db *sql.DB
}

func NewSQLite() *SQLite { return &SQLite{} }

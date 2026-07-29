package orders

import "github.com/example/orders/internal/store"

type Repo struct {
	backend *store.SQLite
}

func NewRepo(b *store.SQLite) *Repo { return &Repo{backend: b} }

// Service layer — Container candidate. Evidence: presence of exported type
// `Service`, presence of Handler, imports internal/store.
package orders

import "github.com/example/orders/internal/store"

type Service struct {
	repo *store.SQLite
}

func NewService(repo *store.SQLite) *Service { return &Service{repo: repo} }

func Handler(s *store.SQLite) http.HandlerFunc {
	svc := NewService(s)
	return func(w http.ResponseWriter, r *http.Request) { _, _ = svc, w, r }
}

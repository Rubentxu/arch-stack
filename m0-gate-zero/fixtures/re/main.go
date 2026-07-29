// Tiny fixture entry point — Go syntax, intentionally tiny. The Gate Zero
// runner will recognise this as a Container element via evidence-discipline
// parsing. It is NOT parsed by an LLM in Gate Zero — it is recognised by
// deterministic shape (file name + function signature + imports).
package main

import (
	"net/http"

	"github.com/example/orders/internal/orders"
	"github.com/example/orders/internal/store"
)

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("/orders", orders.Handler(store.NewSQLite()))
	http.ListenAndServe(":8080", mux)
}

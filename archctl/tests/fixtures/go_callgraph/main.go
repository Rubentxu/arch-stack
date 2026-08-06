// Minimal Go fixture for the call-graph APPLY path smoke test.
// Covers: function declaration, pointer-receiver method, value-receiver
// method, nested func_literal (calls attributed to enclosing named
// function), package-qualified call (fmt.Println), selector method call
// (s.Save()), main and init as regular function nodes.
package main

import "fmt"

func greet(name string) string {
	fmt.Println("hello", name)
	return "hi " + name
}

type Server struct{ name string }

func (s *Server) Save() error {
	return nil
}

func (s Server) Name() string {
	return s.name
}

func handler() {
	fn := func() {
		greet("anon")
	}
	fn()
}

func main() {
	s := &Server{name: "x"}
	s.Save()
	handler()
}

func init() {
	_ = greet("init")
}

// Benchmark reference (O7.1): Go net/http hello world on port 8083.
package main

import "net/http"

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/plain")
		w.Write([]byte("hello\n"))
	})
	http.ListenAndServe("127.0.0.1:8083", nil)
}

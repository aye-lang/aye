/* Integer widths through `@cimport`: `size_t`, `int64_t` and `uint64_t`
   map to `usize`, `i64` and `u64` by name, whatever the host's headers
   define them as; `long` becomes `c_long`. One struct per field, so
   `@sizeOf` gives each field's width. */

#include <stddef.h>
#include <stdint.h>

struct w_size { size_t v; };
struct w_long { long v; };
struct w_int64 { int64_t v; };
struct w_uint64 { uint64_t v; };

struct widths {
    size_t size;
    long l;
    int64_t s64;
    uint64_t big;
};

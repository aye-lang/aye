/* The @cimport golden test: one of each kind of C declaration that
   `@cimport` maps, and some that it skips. The functions are libc's, so
   the program links without a library of its own. */

#include <stddef.h>

#define SHAPES_VERSION 3
#define SHAPES_NAME "shapes"
#define SHAPES_SCALE 2.5
#define SHAPES_HALF 0.5f
#define SHAPES_MASK (1u << 4 | 1u)
#define SHAPES_BIG 0x100000000L
#define SHAPES_NEG (-SHAPES_VERSION * 2)
#define SHAPES_LETTER 'A'
#define SHAPES_TWICE(x) ((x) * 2)
#define SHAPES_NOTHING ((void *)0)
#define SHAPES_EMPTY
#define SHAPES_TYPE unsigned long

enum color { RED, GREEN = 5, BLUE };
typedef enum { SMALL = -1, LARGE = 1 } size_class;

struct point {
    int x;
    int y;
};
typedef struct point point;

typedef int (*compare_fn)(const void *a, const void *b);

typedef struct {
    struct point corners[2];
    enum color color;
    const char *label;
    union {
        int id;
        float weight;
    } tag;
    union {
        long handle;
        double ratio;
    };
    unsigned char fn;
    double match;
    char name[8];
    _Bool visible;
    void (*done)(int code);
} rect;

union number {
    int i;
    double d;
    unsigned char bytes[8];
};

/* Only declared: opaque. */
struct node;
struct node *node_first(struct node *list);

/* Bit-fields and packing: opaque. */
struct flags {
    unsigned ready : 1;
    unsigned count : 7;
};
struct __attribute__((packed)) packed_pair {
    char c;
    int i;
};

/* A struct tag and a function with one name. */
struct stat_like {
    long size;
};
long stat_like(const struct stat_like *s);

struct tm {
    int tm_sec;
    int tm_min;
    int tm_hour;
    int tm_mday;
    int tm_mon;
    int tm_year;
    int tm_wday;
    int tm_yday;
    int tm_isdst;
    long tm_gmtoff;
    char *tm_zone;
};
typedef long time_t;
struct tm *gmtime_r(const time_t *clock, struct tm *result);

typedef struct {
    int quot;
    int rem;
} div_t;
div_t div(int num, int den);

int abs(int n);
double hypot(double x, double y);
size_t strlen(const char *s);
void qsort(void *base, size_t n, size_t size, compare_fn cmp);
int snprintf(char *buf, size_t size, const char *format, ...);
unsigned short swap_bytes(unsigned short v, short s, long long w, unsigned long long z, signed char *p);

/* Skipped: a variable, an inline function and a function named like an
   Ore keyword. */
extern int shapes_count;
static inline int twice(int x) { return 2 * x; }
int defer(int x);

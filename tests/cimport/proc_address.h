/* Function pointer typedefs in the style of a GL or EGL header: each
   entry point's `PFN...PROC` type, which `@cimport` declares as an
   optional function type. */

typedef int (*PFNABSPROC)(int value);
typedef long (*PFNLABSPROC)(long value);

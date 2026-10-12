/* MicroPython configuration for the Wikipedia extension: a WASI command (Volant ABI 2) that runs one
 * request and exits. The garbage collector is off, so objects come from wasi-libc's malloc and are
 * never freed; each run gets a fresh instance whose memory Volant caps at 16 MiB. Leaving the GC out
 * also avoids conservative stack scanning, which cannot see values held in WebAssembly locals.
 * Exceptions use MicroPython's setjmp-based NLR, which wasi-sdk lowers to legacy WebAssembly
 * exception handling. */

#include <port/mpconfigport_common.h>

#define MICROPY_CONFIG_ROM_LEVEL                (MICROPY_CONFIG_ROM_LEVEL_CORE_FEATURES)

#define MICROPY_ENABLE_COMPILER                 (1)
#define MICROPY_ENABLE_GC                       (0)
#define MICROPY_PY_GC                           (0)
#define MICROPY_ENABLE_FINALISER                (0)
#define MICROPY_NLR_SETJMP                      (1)
#define MICROPY_STACK_CHECK                     (1)
#define MICROPY_LONGINT_IMPL                    (MICROPY_LONGINT_IMPL_MPZ)
#define MICROPY_FLOAT_IMPL                      (MICROPY_FLOAT_IMPL_DOUBLE)
#define MICROPY_PY_BUILTINS_STR_UNICODE         (1)
#define MICROPY_PY_BUILTINS_STR_UNICODE_CHECK   (1)
#define MICROPY_PY_BUILTINS_STR_OP_MODULO       (1)
#define MICROPY_PY_JSON                         (1)
#define MICROPY_PY_JSON_SEPARATORS              (1)
#define MICROPY_PY_SYS                          (0)
#define MICROPY_PY_IO                           (1)
#define MICROPY_PY_OS                           (0)
#define MICROPY_PY_TIME                         (0)
#define MICROPY_PY_RANDOM                       (0)
#define MICROPY_PY_STRUCT                       (0)
#define MICROPY_PY_ERRNO                        (0)
#define MICROPY_PY_SELECT                       (0)
#define MICROPY_PY_ARRAY                        (0)
#define MICROPY_PY_COLLECTIONS                  (0)
#define MICROPY_PY_MATH                         (0)
#define MICROPY_PY_CMATH                        (0)
#define MICROPY_PY_MICROPYTHON                  (0)
#define MICROPY_PY_BUILTINS_HELP                (0)
#define MICROPY_PY_BUILTINS_INPUT               (0)
#define MICROPY_HELPER_REPL                     (0)
#define MICROPY_ENABLE_SOURCE_LINE              (1)
#define MICROPY_ERROR_REPORTING                 (MICROPY_ERROR_REPORTING_NORMAL)

/* WASI command entry point for the Wikipedia extension (Volant ABI 2). Reads Volant's request JSON
 * from stdin, runs it through respond() in the embedded wikipedia.py on MicroPython, and writes the
 * reply JSON to stdout. A Python failure that escapes respond() becomes one explaining row, so Volant
 * always receives a reply it can show. */
#include <string.h>
#include <unistd.h>

#include "py/compile.h"
#include "py/builtin.h"
#include "py/lexer.h"
#include "py/mperrno.h"
#include "py/nlr.h"
#include "py/parse.h"
#include "py/runtime.h"
#include "py/stackctrl.h"

#include "wikipedia_source.h"

#define INPUT_LIMIT (64 * 1024)
#define STACK_LIMIT (896 * 1024)

static const char SOURCE_NAME[] = "wikipedia.py";
static const char ENTRY_POINT[] = "respond";
static const char FALLBACK[] = "{\"items\": [{\"title\": \"The Wikipedia extension could not finish.\"}]}";

static char input[INPUT_LIMIT];

/* Reads all of stdin, up to INPUT_LIMIT bytes, and returns how many bytes arrived. */
static size_t read_input(void) {
    size_t used = 0;
    while (used < INPUT_LIMIT) {
        ssize_t count = read(0, input + used, INPUT_LIMIT - used);
        if (count <= 0) {
            break;
        }
        used += (size_t)count;
    }
    return used;
}

/* Writes every byte of text to stdout, retrying short writes. */
static void write_output(const char *text, size_t length) {
    while (length > 0) {
        ssize_t count = write(1, text, length);
        if (count <= 0) {
            return;
        }
        text += count;
        length -= (size_t)count;
    }
}

/* Compiles and runs the embedded module source, then calls its entry point with the request text and
 * returns the reply string object. Raises through NLR on any Python error. */
static mp_obj_t run_script(size_t input_length) {
    mp_lexer_t *lexer = mp_lexer_new_from_str_len(qstr_from_str(SOURCE_NAME), wikipedia_source, sizeof(wikipedia_source) - 1, 0);
    qstr source_name = lexer->source_name;
    mp_parse_tree_t tree = mp_parse(lexer, MP_PARSE_FILE_INPUT);
    mp_call_function_0(mp_compile(&tree, source_name, false));
    mp_obj_t respond = mp_load_global(qstr_from_str(ENTRY_POINT));
    return mp_call_function_1(respond, mp_obj_new_str(input, input_length));
}

/* Runs one request: read, respond, write, exit 0. */
int main(void) {
    size_t input_length = read_input();
    int stack_top;
    mp_stack_set_top(&stack_top);
    mp_stack_set_limit(STACK_LIMIT);
    mp_init();
    nlr_buf_t nlr;
    if (nlr_push(&nlr) == 0) {
        size_t length;
        const char *reply = mp_obj_str_get_data(run_script(input_length), &length);
        write_output(reply, length);
        nlr_pop();
    } else {
        write_output(FALLBACK, sizeof(FALLBACK) - 1);
    }
    mp_deinit();
    return 0;
}

/* Called if an exception is raised outside every NLR handler; there is nothing left to unwind to. */
void nlr_jump_fail(void *val) {
    (void)val;
    write_output(FALLBACK, sizeof(FALLBACK) - 1);
    _exit(0);
}

/* Reports that no module file exists, so imports resolve only to built-in modules such as json. */
mp_import_stat_t mp_import_stat(const char *path) {
    (void)path;
    return MP_IMPORT_STAT_NO_EXIST;
}

/* There is no filesystem, so reading Python source from a file always fails. */
mp_lexer_t *mp_lexer_new_from_file(qstr filename) {
    (void)filename;
    mp_raise_OSError(MP_ENOENT);
}

/* There is no filesystem, so open() always fails. */
static mp_obj_t builtin_open(size_t n_args, const mp_obj_t *args, mp_map_t *kwargs) {
    (void)n_args;
    (void)args;
    (void)kwargs;
    mp_raise_OSError(MP_ENOENT);
}
MP_DEFINE_CONST_FUN_OBJ_KW(mp_builtin_open_obj, 1, builtin_open);

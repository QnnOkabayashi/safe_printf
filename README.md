# Safe `printf`

A command line interface tool to read C source code files and check for vulnerable uses of `printf`, `sprintf`, and `snprintf`.
Additionally, the tool can rewrite the source code files.
Use the following for help on how to use:
```
safe_printf --help
```

## Features
* Catches instances of non string literals as the format string of formatting functions.
* If type casts on arguments are present, will check that they match the specifiers in the format string.
* `--typecast` option rewrites the file to add type casts matching the type of the specifier.
* [In progress] `--optimized` option rewrites the file with optimized print calls by manually interpolating format strings within the tool.

## Examples

Take some C source code for example:

_Filename: `examples/readme.c`_
```c
#include <stdio.h>

int main() {
    printf("Hello, world!");
    printf("Balance: $%d.", 100);
    printf ( " s p a c e %d ", 100 );

    char input[1024] = {0};
    char name[] = "Quinn";

    // We can write to buffers
    snprintf(input, 1023, "Hello, %s!", name);
    printf("%s", input);

    // Not actually calling printf here
    void* function_ptr = printf;

    // Don't read comments
    // printf("%s, %s", (int) 1, (int) 1);

    // And don't change contents of strings
    char in_a_string[] = "printf(\"hello\")";

    return 0;
}
```

To get typecasts, run the following:
```
safe_printf examples/readme.c --typecast examples/readme_typecast.c
```

_Filename: `examples/readme_typecast.c`_
```c
#include <stdio.h>

int main() {
    printf("Hello, world!");
    printf("Balance: $%d.", (int) (100));
    printf(" s p a c e %d ", (int) (100));

    char input[1024] = {0};
    char name[] = "Quinn";

    // We can write to buffers
    snprintf((char* restrict) (input), (size_t) (1023), "Hello, %s!", (char*) (name));
    printf("%s", (char*) (input));

    // Assign printf to a fn ptr without actually calling
    void* function_ptr = printf;

    // printf in comments is ignored
    // printf("%s, %s", (int) 1, (int) 1);

    // and printf in strings is also ignored
    char in_a_string[] = "printf(\"hello\")";

    return 0;
}
```

Optimized is still a work in progress, but the idea is that there will eventually
be an associated C library that will provide bindings to `safe_*` print functions,
taking advantage of the fact that format strings are already interpolated by the tool.

To get optimized formatting, run the following:
```
safe_printf examples/readme.c --optimize examples/readme_optimize.c
```
_Filename: `examples/readme_optimize.c`_
```c
#include <stdio.h>

int main() {
    safe_printf(1, { "Hello, world!" });
    safe_printf(3, { "Balance: $", { (int) (100), fmt_int }, "." });
    safe_printf(3, { " s p a c e ", { (int) (100), fmt_int }, " " });

    char input[1024] = {0};
    char name[] = "Quinn";

    // We can write to buffers
    safe_snprintf((char* restrict) (input), (size_t) (1023), 3, { "Hello, ", { (char*) (name), fmt_string }, "!" });
    safe_printf(3, { "", { (char*) (input), fmt_string }, "" });

    // Assign printf to a fn ptr without actually calling
    void* function_ptr = printf;

    // printf in comments is ignored
    // printf("%s, %s", (int) 1, (int) 1);

    // and printf in strings is also ignored
    char in_a_string[] = "printf(\"hello\")";

    return 0;
}
```

## Errors
Another focus of this application is helpful error messages.
Take _`examples/unsafe.c`_ for example:
```c
#include <stdio.h>

int main() {
    printf("I will echo\n");
    char credit_card[] = "123";
    while(1) {
        char input[1024] = {0};
        setvbuf(stdin, NULL, _IONBF, 0);
        printf("> ");
        fgets(input, 1023, stdin);

        printf("normal: %s", input);
        printf("unsafe: ");
        printf(input, 1); // adding an arg here evades even -Wpedantic...
        printf("%s is %s", input); // gcc catches this though
    }
}

```
Running `safe_printf` on this program prints a helpful error to the console:
```
Error:
  × Source code contains errors.

Error:
  × Format string isn't a string literal, this is potentially an overflow vulnerability!
    ╭─[examples/unsafe.c:13:1]
 13 │         printf("unsafe: ");
 14 │         printf(input, 1); // adding an arg here evades even -Wpedantic...
    ·                ──┬──
    ·                  ╰── not a string literal
 15 │         printf("%s is %s", input); // gcc catches this though
    ╰────
  help: To safely print a string, use `printf("%s", input)` instead.
Error:
  × Excess specifiers, this will read arbitrary data off the stack!
    ╭─[examples/unsafe.c:14:1]
 14 │         printf(input, 1); // adding an arg here evades even -Wpedantic...
 15 │         printf("%s is %s", input); // gcc catches this though
    ·                ─────┬───────┬───
    ·                     │       ╰── not enough arguments
    ·                     ╰── 1 too many specifiers
 16 │     }
    ╰────
  help: Add an argument or remove a specifier.
```
> Note: some markdown renders may render the lines weirdly, but they show up straight (and with pretty colors!) in the terminal.

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

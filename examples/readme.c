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

    // Assign printf to a fn ptr without actually calling
    void* function_ptr = printf;

    // printf in comments is ignored
    // printf("%s, %s", (int) 1, (int) 1);

    // and printf in strings is also ignored
    char in_a_string[] = "printf(\"hello\")";

    return 0;
}

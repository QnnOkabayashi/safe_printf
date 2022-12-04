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
        printf("%s", input, 42);
    }
}

#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

int main() {
    void *x = malloc(1000);

    // Print the first 10 bytes of thze memory address of x
    printf("Bytes of x: ");
    for (int i = 10; i < 1000; i++) {
        // ((char*)x)[i] = 0x41;
        printf("%02x ", ((char *)x)[i]);
    }
    printf("\n");

    // printf("Hello, World! %p\n", x);
    printf("Hello, World! %p\n", x);
    // printf("Hello, World! %p\n", x);
    return 0;
}
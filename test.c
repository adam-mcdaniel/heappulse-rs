#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

#define alloc(size) mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)

int main() {
    void *x = alloc(5000);

    // Print the first 10 bytes of thze memory address of x
    printf("Bytes of x: ");
    for (int i = 10; i < 1000; i++) {
        printf("%02x ", ((char *)x)[i]);
        ((char*)x)[i] = 0x41;
        printf("%02x ", ((char *)x)[i]);
    }
    printf("\n");

    // printf("Hello, World! %p\n", x);
    printf("Hello, World! %p\n", x);
    // printf("Hello, World! %p\n", x);
    return 0;
}
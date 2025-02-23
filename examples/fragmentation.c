#include <stdio.h>
#include <stdlib.h>

#define N 10000

int main() {
    void *ptrs[N];

    // Allocate memory
    for (int i = 0; i < N; i++) {
        ptrs[i] = malloc(rand() % 1024 + 1); // Random sizes 1-1024 bytes
    }

    // Free every other allocation
    for (int i = 0; i < N; i += 2) {
        free(ptrs[i]);
    }

    // Allocate again to test fragmentation handling
    for (int i = 0; i < N; i += 2) {
        ptrs[i] = malloc(rand() % 1024 + 1);
    }

    // Cleanup
    for (int i = 0; i < N; i++) {
        free(ptrs[i]);
    }

    printf("Memory fragmentation test completed.\n");
    return 0;
}
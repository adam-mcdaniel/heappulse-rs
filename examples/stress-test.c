#include <stdio.h>
#include <stdlib.h>

int main() {
    for (size_t i = 0; i < 1000000; i++) {
        void *ptr = malloc(128);
        if (!ptr) {
            fprintf(stderr, "malloc failed!\n");
            return 1;
        }
        free(ptr);
    }
    printf("Completed 1,000,000 malloc/free cycles.\n");
    return 0;
}
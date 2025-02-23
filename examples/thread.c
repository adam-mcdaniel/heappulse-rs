#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>

#define THREADS 10
#define ALLOCS 10000

void *worker(void *arg) {
    for (int i = 0; i < ALLOCS; i++) {
        void *ptr = malloc(64);
        free(ptr);
    }
    return NULL;
}

int main() {
    pthread_t threads[THREADS];

    for (int i = 0; i < THREADS; i++) {
        pthread_create(&threads[i], NULL, worker, NULL);
    }

    for (int i = 0; i < THREADS; i++) {
        pthread_join(threads[i], NULL);
    }

    printf("Multi-threaded malloc test completed.\n");
    return 0;
}
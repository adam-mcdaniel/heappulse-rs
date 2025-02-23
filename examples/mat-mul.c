#include <stdio.h>
#include <stdlib.h>

#define N 512  // Change for larger matrix

int main() {
    double **A = malloc(N * sizeof(double *));
    double **B = malloc(N * sizeof(double *));
    double **C = malloc(N * sizeof(double *));

    for (int i = 0; i < N; i++) {
        A[i] = malloc(N * sizeof(double));
        B[i] = malloc(N * sizeof(double));
        C[i] = malloc(N * sizeof(double));
    }

    // Initialize matrices
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            A[i][j] = i + j;
            B[i][j] = i - j;
            C[i][j] = 0;
        }

    // Matrix multiplication
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            for (int k = 0; k < N; k++)
                C[i][j] += A[i][k] * B[k][j];

    printf("Matrix multiplication completed.\n");

    // Free memory
    for (int i = 0; i < N; i++) {
        free(A[i]);
        free(B[i]);
        free(C[i]);
    }

    free(A);
    free(B);
    free(C);

    return 0;
}
#include <stdio.h>

int main(void) {
  int a = 0;
  scanf("%d", &a);
  if (a < 10000) {
    printf("%d", a + 1);
  } else {
    int *p = NULL;
    *p = 1;
  }
}
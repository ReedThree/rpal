#include <stdio.h>

int main(void) {
  int a = 0;
  scanf("%d", &a);
  if (a < 2100001000) {
    printf("%d", a + 1);
  } else {
    printf("%d", a);
  }
}
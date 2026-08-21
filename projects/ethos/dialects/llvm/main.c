#include <stdio.h>

extern int read_global_cfg();

int main()
{
    int config_value = read_global_cfg();
    printf("The value of global_cfg is: %d\n", config_value);
    return 0;
}

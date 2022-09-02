// Defaults
#define configMINIMAL_STACK_SIZE 1024
#define configMAX_SYSCALL_INTERRUPT_PRIORITY 4
#define configTOTAL_HEAP_SIZE 4096
#define configMAX_PRIORITIES 5
#define configUSE_16_BIT_TICKS 0
#define configTICK_RATE_HZ 1000
#define configKERNEL_INTERRUPT_PRIORITY 1
#define configCPU_CLOCK_HZ 20000000
#define configQUEUE_REGISTRY_SIZE 1
#define configTIMER_TASK_STACK_DEPTH configMINIMAL_STACK_SIZE

// Required features.
#define configUSE_IDLE_HOOK 1
#define configUSE_TICK_HOOK 1
#define configUSE_PREEMPTION 1
#define configUSE_MUTEXES 1
#define configUSE_RECURSIVE_MUTEXES 1
#define configSUPPORT_STATIC_ALLOCATION 1
#define configUSE_TASK_NOTIFICATIONS 1

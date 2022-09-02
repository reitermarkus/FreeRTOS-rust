#ifndef FREERTOS_CONFIG_H
#define FREERTOS_CONFIG_H

// Defaults
#ifndef configMINIMAL_STACK_SIZE
#define configMINIMAL_STACK_SIZE 1024
#endif

#ifndef configTIMER_TASK_STACK_DEPTH
#define configTIMER_TASK_STACK_DEPTH configMINIMAL_STACK_SIZE
#endif

#ifndef configMAX_SYSCALL_INTERRUPT_PRIORITY
#define configMAX_SYSCALL_INTERRUPT_PRIORITY 4
#endif

#ifndef configTOTAL_HEAP_SIZE
#define configTOTAL_HEAP_SIZE 4096
#endif

#ifndef configMAX_PRIORITIES
#define configMAX_PRIORITIES 5
#endif

#ifndef configUSE_16_BIT_TICKS
#define configUSE_16_BIT_TICKS 0
#endif

#ifndef configTICK_RATE_HZ
#define configTICK_RATE_HZ 1000
#endif

#ifndef configKERNEL_INTERRUPT_PRIORITY
#define configKERNEL_INTERRUPT_PRIORITY 1
#endif

#ifndef configCPU_CLOCK_HZ
#define configCPU_CLOCK_HZ 20000000
#endif

#ifndef configQUEUE_REGISTRY_SIZE
#define configQUEUE_REGISTRY_SIZE 8
#endif

// Required features.
#define configUSE_IDLE_HOOK 1
#define configUSE_TICK_HOOK 1
#define configUSE_PREEMPTION 1
#define configUSE_MUTEXES 1
#define configUSE_RECURSIVE_MUTEXES 1
#define configSUPPORT_STATIC_ALLOCATION 1
#define configUSE_TASK_NOTIFICATIONS 1
#define INCLUDE_uxTaskPriorityGet 1
#define INCLUDE_vTaskDelay 1
#define INCLUDE_vTaskPrioritySet 1
#define INCLUDE_vTaskDelayUntil 1
#define INCLUDE_vTaskSuspend 1
#define INCLUDE_vTaskDelete 1

#endif

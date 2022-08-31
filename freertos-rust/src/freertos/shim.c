#include "FreeRTOS.h"
#include "task.h"
#include "timers.h"
#include "queue.h"
#include "semphr.h"

// Just for testing
void freertos_rs_invoke_configASSERT() {
	configASSERT(0);
}

uint8_t freertos_rs_sizeof(uint8_t _type) {
	switch (_type) {
		case 0:
			return sizeof(void*);
			break;
		case 1:
			return sizeof(char*);
			break;
		case 2:
			return sizeof(char);
			break;
		case 10:
			return sizeof(BaseType_t);
			break;
		case 11:
			return sizeof(UBaseType_t);
			break;
		case 12:
			return sizeof(TickType_t);
			break;
		case 20:
			return sizeof(TaskHandle_t);
			break;
		case 21:
			return sizeof(QueueHandle_t);
			break;
		case 22:
			return sizeof(SemaphoreHandle_t);
			break;
		case 23:
			return sizeof(TaskFunction_t);
			break;
		case 24:
			return sizeof(TimerHandle_t);
			break;
		case 25:
			return sizeof(TimerCallbackFunction_t);
			break;
		case 30:
			return sizeof(TaskStatus_t);
			break;
		case 31:
			return sizeof(eTaskState);
			break;
		case 32:
			return sizeof(unsigned long);
			break;
		case 33:
			return sizeof(unsigned short);
			break;
		default:
			return 0;
	}
}

unsigned long freertos_rs_get_configCPU_CLOCK_HZ() {
  return configCPU_CLOCK_HZ;
}

UBaseType_t freertos_rs_get_stack_high_water_mark(TaskHandle_t task) {
#if (INCLUDE_uxTaskGetStackHighWaterMark == 1)
	return uxTaskGetStackHighWaterMark(task);
#else
	return 0;
#endif
}

void freertos_rs_yield_from_isr(BaseType_t x) {
	portYIELD_FROM_ISR(x);
}

BaseType_t freertos_rs_task_notify_indexed(TaskHandle_t task, UBaseType_t index, uint32_t value, eNotifyAction eAction) {
	return xTaskNotifyIndexed(task, index, value, eAction);
}

BaseType_t freertos_rs_task_notify_indexed_from_isr(TaskHandle_t task, UBaseType_t index, uint32_t value, eNotifyAction eAction, BaseType_t* xHigherPriorityTaskWoken) {
	return xTaskNotifyIndexedFromISR(task, index, value, eAction, xHigherPriorityTaskWoken);
}

#if (configUSE_TIMERS == 1)

TimerHandle_t freertos_rs_timer_create(const char * const name, uint8_t name_len, const TickType_t period,
		uint8_t auto_reload, void * const timer_id, TimerCallbackFunction_t callback)
{
	char c_name[configMAX_TASK_NAME_LEN] = {0};
	for (int i = 0; i < name_len; i++) {
		c_name[i] = name[i];

		if (i == configMAX_TASK_NAME_LEN - 1) {
			break;
		}
	}

	UBaseType_t timer_auto_reload = pdFALSE;
	if (auto_reload == 1) {
		timer_auto_reload = pdTRUE;
	}

	TimerHandle_t handle = xTimerCreate(c_name, period, timer_auto_reload, timer_id, callback);
	return handle;
}

#endif

void freertos_rs_enter_critical() {
	taskENTER_CRITICAL();
}

void freertos_rs_exit_critical() {
	taskEXIT_CRITICAL();
}

#include "FreeRTOS.h"
#include "task.h"
#include "timers.h"
#include "queue.h"
#include "semphr.h"

// Fix constants for bindgen.

static const uint16_t configMINIMAL_STACK_SIZE_TMP = configMINIMAL_STACK_SIZE;
#undef configMINIMAL_STACK_SIZE
const uint16_t configMINIMAL_STACK_SIZE = configMINIMAL_STACK_SIZE_TMP;

static const uint16_t configTIMER_TASK_STACK_DEPTH_TMP = configTIMER_TASK_STACK_DEPTH;
#undef configTIMER_TASK_STACK_DEPTH
const uint16_t configTIMER_TASK_STACK_DEPTH = configTIMER_TASK_STACK_DEPTH_TMP;

static const BaseType_t queueSEND_TO_BACK_TMP = queueSEND_TO_BACK;
#undef queueSEND_TO_BACK
const BaseType_t queueSEND_TO_BACK = queueSEND_TO_BACK_TMP;

static const BaseType_t queueSEND_TO_FRONT_TMP = queueSEND_TO_FRONT;
#undef queueSEND_TO_FRONT
const BaseType_t queueSEND_TO_FRONT = queueSEND_TO_FRONT_TMP;

static const TickType_t semGIVE_BLOCK_TIME_TMP = semGIVE_BLOCK_TIME;
#undef semGIVE_BLOCK_TIME
const TickType_t semGIVE_BLOCK_TIME = semGIVE_BLOCK_TIME_TMP;

static const uint8_t queueQUEUE_TYPE_BASE_TMP = queueQUEUE_TYPE_BASE;
#undef queueQUEUE_TYPE_BASE
const uint8_t queueQUEUE_TYPE_BASE = queueQUEUE_TYPE_BASE_TMP;

static const uint8_t queueQUEUE_TYPE_BINARY_SEMAPHORE_TMP = queueQUEUE_TYPE_BINARY_SEMAPHORE;
#undef queueQUEUE_TYPE_BINARY_SEMAPHORE
const uint8_t queueQUEUE_TYPE_BINARY_SEMAPHORE = queueQUEUE_TYPE_BINARY_SEMAPHORE_TMP;

static const uint8_t queueQUEUE_TYPE_MUTEX_TMP = queueQUEUE_TYPE_MUTEX;
#undef queueQUEUE_TYPE_MUTEX
const uint8_t queueQUEUE_TYPE_MUTEX = queueQUEUE_TYPE_MUTEX_TMP;

static const uint8_t queueQUEUE_TYPE_RECURSIVE_MUTEX_TMP = queueQUEUE_TYPE_RECURSIVE_MUTEX;
#undef queueQUEUE_TYPE_RECURSIVE_MUTEX
const uint8_t queueQUEUE_TYPE_RECURSIVE_MUTEX = queueQUEUE_TYPE_RECURSIVE_MUTEX_TMP;

static const UBaseType_t semSEMAPHORE_QUEUE_ITEM_LENGTH_TMP = semSEMAPHORE_QUEUE_ITEM_LENGTH;
#undef semSEMAPHORE_QUEUE_ITEM_LENGTH
const UBaseType_t semSEMAPHORE_QUEUE_ITEM_LENGTH = semSEMAPHORE_QUEUE_ITEM_LENGTH_TMP;

static const BaseType_t queueOVERWRITE_TMP = queueOVERWRITE;
#undef queueOVERWRITE
const BaseType_t queueOVERWRITE = queueOVERWRITE_TMP;

static const BaseType_t pdFALSE_TMP = pdFALSE;
#undef pdFALSE
const BaseType_t pdFALSE = pdFALSE_TMP;

static const BaseType_t pdTRUE_TMP = pdTRUE;
#undef pdTRUE
const BaseType_t pdTRUE = pdTRUE_TMP;

static const BaseType_t tmrCOMMAND_DELETE_TMP = tmrCOMMAND_DELETE;
#undef tmrCOMMAND_DELETE
const BaseType_t tmrCOMMAND_DELETE = tmrCOMMAND_DELETE_TMP;

static const BaseType_t tmrCOMMAND_STOP_TMP = tmrCOMMAND_STOP;
#undef tmrCOMMAND_STOP
const BaseType_t tmrCOMMAND_STOP = tmrCOMMAND_STOP_TMP;

static const BaseType_t tmrCOMMAND_STOP_FROM_ISR_TMP = tmrCOMMAND_STOP_FROM_ISR;
#undef tmrCOMMAND_STOP_FROM_ISR
const BaseType_t tmrCOMMAND_STOP_FROM_ISR = tmrCOMMAND_STOP_FROM_ISR_TMP;

static const BaseType_t tmrCOMMAND_CHANGE_PERIOD_TMP = tmrCOMMAND_CHANGE_PERIOD;
#undef tmrCOMMAND_CHANGE_PERIOD
const BaseType_t tmrCOMMAND_CHANGE_PERIOD = tmrCOMMAND_CHANGE_PERIOD_TMP;

static const BaseType_t tmrCOMMAND_CHANGE_PERIOD_FROM_ISR_TMP = tmrCOMMAND_CHANGE_PERIOD_FROM_ISR;
#undef tmrCOMMAND_CHANGE_PERIOD_FROM_ISR
const BaseType_t tmrCOMMAND_CHANGE_PERIOD_FROM_ISR = tmrCOMMAND_CHANGE_PERIOD_FROM_ISR_TMP;


// Just for testing
void freertos_rs_invoke_configASSERT() {
	configASSERT(0);
}

void freertos_rs_vTaskStartScheduler() {
	vTaskStartScheduler();
}

BaseType_t freertos_rt_xTaskGetSchedulerState(void) {
  return xTaskGetSchedulerState();
}

void *freertos_rs_pvPortMalloc(size_t xWantedSize) {
	return pvPortMalloc(xWantedSize);
}

void freertos_rs_vPortFree(void *pv) {
	vPortFree(pv);
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


			break;
		default:
			return 0;
	}
}

#if (INCLUDE_vTaskDelayUntil == 1)
void freertos_rs_vTaskDelayUntil(TickType_t *pxPreviousWakeTime, TickType_t xTimeIncrement) {
	vTaskDelayUntil(pxPreviousWakeTime, xTimeIncrement);
}
#endif

#if (INCLUDE_vTaskDelay == 1)
void freertos_rs_vTaskDelay(TickType_t xTicksToDelay) {
	vTaskDelay(xTicksToDelay);
}
#endif

TickType_t freertos_rs_xTaskGetTickCount() {
	return xTaskGetTickCount();
}

UBaseType_t freertos_rs_get_system_state(TaskStatus_t * const pxTaskStatusArray, const UBaseType_t uxArraySize, uint32_t * const pulTotalRunTime) {
	return uxTaskGetSystemState(pxTaskStatusArray, uxArraySize, pulTotalRunTime);
}

unsigned long freertos_rs_get_configCPU_CLOCK_HZ() {
  return configCPU_CLOCK_HZ;
}

static const TickType_t portTICK_PERIOD_MS_TMP = portTICK_PERIOD_MS;
#undef portTICK_PERIOD_MS
const TickType_t portTICK_PERIOD_MS = portTICK_PERIOD_MS_TMP;

UBaseType_t freertos_rs_get_number_of_tasks() {
	return uxTaskGetNumberOfTasks();
}

#if (configUSE_RECURSIVE_MUTEXES == 1)
QueueHandle_t freertos_rs_semaphore_create_recursive_mutex() {
	return xSemaphoreCreateRecursiveMutex();
}

UBaseType_t freertos_rs_semaphore_take_recursive(QueueHandle_t mutex, UBaseType_t max) {
	if (xSemaphoreTakeRecursive(mutex, max) == pdTRUE) {
		return 0;
	}

	return 1;
}
UBaseType_t freertos_rs_semaphore_give_recursive(QueueHandle_t mutex) {
	if (xSemaphoreGiveRecursive(mutex) == pdTRUE) {
		return 0;
	} else {
		return 1;
	}
}
#endif

QueueHandle_t freertos_rs_semaphore_create_mutex() {
	return xSemaphoreCreateMutex();
}

QueueHandle_t freertos_rs_semaphore_create_binary() {
	return xSemaphoreCreateBinary();
}

QueueHandle_t freertos_rs_semaphore_create_binary_static(StaticSemaphore_t* pxStaticSemaphore) {
	return xSemaphoreCreateBinaryStatic(pxStaticSemaphore);
}

QueueHandle_t freertos_rs_semaphore_create_counting(UBaseType_t max, UBaseType_t initial) {
	return xSemaphoreCreateCounting(max, initial);
}

QueueHandle_t freertos_rs_semaphore_create_counting_static(UBaseType_t max, UBaseType_t initial, StaticSemaphore_t* pxSemaphoreBuffer) {
	return xSemaphoreCreateCountingStatic(max, initial, pxSemaphoreBuffer);
}

void freertos_rs_semaphore_delete(QueueHandle_t semaphore) {
	vSemaphoreDelete(semaphore);
}

UBaseType_t freertos_rs_semaphore_take(QueueHandle_t mutex, UBaseType_t max) {
	if (xSemaphoreTake(mutex, max) == pdTRUE) {
		return 0;
	}

	return 1;
}

UBaseType_t freertos_rs_semaphore_give(QueueHandle_t mutex) {
	if (xSemaphoreGive(mutex) == pdTRUE) {
		return 0;
	}

	return 1;
}

UBaseType_t freertos_rs_take_semaphore_isr(QueueHandle_t semaphore, BaseType_t* xHigherPriorityTaskWoken) {
	if (xSemaphoreTakeFromISR(semaphore, xHigherPriorityTaskWoken) == pdTRUE) {
		return 0;
	}

	return 1;
}

UBaseType_t freertos_rs_semaphore_give_from_isr(QueueHandle_t semaphore, BaseType_t* xHigherPriorityTaskWoken) {
	if (xSemaphoreGiveFromISR(semaphore, xHigherPriorityTaskWoken) == pdTRUE) {
		return 0;
	}

	return 1;
}

UBaseType_t freertos_rs_spawn_task(TaskFunction_t entry_point, void* pvParameters, const char * const name, uint8_t name_len, uint16_t stack_size, UBaseType_t priority, TaskHandle_t *const task_handle) {
	char c_name[configMAX_TASK_NAME_LEN] = {0};
	for (int i = 0; i < name_len; i++) {
		c_name[i] = name[i];

		if (i == configMAX_TASK_NAME_LEN - 1) {
			break;
		}
	}

	BaseType_t ret = xTaskCreate(entry_point, c_name, stack_size, pvParameters, priority, task_handle);

	if (ret != pdPASS) {
		return 1;
	}

	configASSERT(task_handle);

	return 0;
}

#if (INCLUDE_vTaskDelete == 1)
void freertos_rs_delete_task(TaskHandle_t task) {
	vTaskDelete(task);
}
#endif

UBaseType_t freertos_rs_get_stack_high_water_mark(TaskHandle_t task) {
#if (INCLUDE_uxTaskGetStackHighWaterMark == 1)
	return uxTaskGetStackHighWaterMark(task);
#else
	return 0;
#endif
}


QueueHandle_t freertos_rs_queue_create(UBaseType_t queue_length, UBaseType_t item_size) {
	return xQueueCreate(queue_length, item_size);
}

QueueHandle_t freertos_rs_queue_create_static(UBaseType_t queue_length, UBaseType_t item_size, uint8_t* pucQueueStorageBuffer, StaticQueue_t* pxQueueBuffer) {
	return xQueueCreateStatic(queue_length, item_size, pucQueueStorageBuffer, pxQueueBuffer);
}

void freertos_rs_queue_delete(QueueHandle_t queue) {
	vQueueDelete(queue);
}

UBaseType_t freertos_rs_queue_send(QueueHandle_t queue, const void* const item, TickType_t max_wait) {
	if (xQueueSend(queue, item, max_wait ) != pdTRUE)
	{
		return 1;
	}

	return 0;
}

UBaseType_t freertos_rs_queue_send_isr(QueueHandle_t queue, const void* const item, BaseType_t* xHigherPriorityTaskWoken) {
	if (xQueueSendFromISR(queue, item, xHigherPriorityTaskWoken) == pdTRUE) {
		return 0;
	}
	return 1;
}

UBaseType_t freertos_rs_queue_receive(QueueHandle_t queue, void* const item, TickType_t max_wait) {
	if ( xQueueReceive( queue, item, max_wait ) != pdTRUE )
	{
		return 1;
	}

	return 0;
}

void freertos_rs_yield_from_isr(BaseType_t x) {
	portYIELD_FROM_ISR(x);
}

static const TickType_t portMAX_DELAY_TMP = portMAX_DELAY;
#undef portMAX_DELAY
const TickType_t portMAX_DELAY = portMAX_DELAY_TMP;

uint32_t freertos_rs_task_notify_take(BaseType_t clear_count, TickType_t wait) {
	return ulTaskNotifyTake(clear_count == 0 ? pdFALSE : pdTRUE, wait);
}

BaseType_t freertos_rs_task_notify_wait(uint32_t ulBitsToClearOnEntry, uint32_t ulBitsToClearOnExit, uint32_t *pulNotificationValue, TickType_t xTicksToWait) {
	if (xTaskNotifyWait(ulBitsToClearOnEntry, ulBitsToClearOnExit, pulNotificationValue, xTicksToWait) == pdTRUE) {
		return 0;
	}

	return 1;
}

BaseType_t freertos_rs_task_notify(TaskHandle_t task, uint32_t value, eNotifyAction eAction) {
	BaseType_t v = xTaskNotify(task, value, eAction);
	if (v != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_task_notify_indexed(TaskHandle_t task, UBaseType_t index, uint32_t value, eNotifyAction eAction) {
	BaseType_t v = xTaskNotifyIndexed(task, index, value, eAction);
	if (v != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_task_notify_from_isr(TaskHandle_t task, uint32_t value, eNotifyAction eAction, BaseType_t* xHigherPriorityTaskWoken) {
	BaseType_t v = xTaskNotifyFromISR(task, value, eAction, xHigherPriorityTaskWoken);
	if (v != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_task_notify_indexed_from_isr(TaskHandle_t task, UBaseType_t index, uint32_t value, eNotifyAction eAction, BaseType_t* xHigherPriorityTaskWoken) {
	BaseType_t v = xTaskNotifyIndexedFromISR(task, index, value, eAction, xHigherPriorityTaskWoken);
	if (v != pdPASS) {
		return 1;
	}
	return 0;
}

#if ( ( INCLUDE_xTaskGetCurrentTaskHandle == 1 ) || ( configUSE_MUTEXES == 1 ) )
TaskHandle_t freertos_rs_get_current_task() {
	return xTaskGetCurrentTaskHandle();
}
#endif

BaseType_t freertos_rs_xTaskResumeAll() {
  return xTaskResumeAll();
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

BaseType_t freertos_rs_timer_start(TimerHandle_t timer, TickType_t block_time) {
	if (xTimerStart(timer, block_time) != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_timer_start_from_isr(TimerHandle_t timer, BaseType_t* xHigherPriorityTaskWoken) {
	if (xTimerStartFromISR(timer, xHigherPriorityTaskWoken) != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_timer_stop(TimerHandle_t timer, TickType_t block_time) {
	if (xTimerStop(timer, block_time) != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_timer_delete(TimerHandle_t timer, TickType_t block_time) {
	if (xTimerDelete(timer, block_time) != pdPASS) {
		return 1;
	}
	return 0;
}

BaseType_t freertos_rs_timer_change_period(TimerHandle_t timer, TickType_t block_time, TickType_t new_period) {
	if (xTimerChangePeriod(timer, new_period, block_time) != pdPASS) {
		return 1;
	}
	return 0;
}

void* freertos_rs_timer_get_id(TimerHandle_t timer) {
	return pvTimerGetTimerID(timer);
}

#endif

void freertos_rs_enter_critical() {
	taskENTER_CRITICAL();
}

void freertos_rs_exit_critical() {
	taskEXIT_CRITICAL();
}

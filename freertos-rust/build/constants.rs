use std::fs::File;
use std::io::{self, Write};

pub fn write_to_file(f: &mut File) -> io::Result<()> {
  let constants = vec![
    ("uint16_t",    "configMINIMAL_STACK_SIZE"),
    ("uint16_t",    "configTIMER_TASK_STACK_DEPTH"),

    ("BaseType_t",  "pdFALSE"),
    ("BaseType_t",  "pdFAIL"),
    ("BaseType_t",  "pdTRUE"),
    ("BaseType_t",  "pdPASS"),
    ("BaseType_t",  "errQUEUE_FULL"),
    ("BaseType_t",  "errQUEUE_EMPTY"),
    ("BaseType_t",  "errCOULD_NOT_ALLOCATE_REQUIRED_MEMORY"),
    ("BaseType_t",  "errQUEUE_BLOCKED"),
    ("BaseType_t",  "errQUEUE_YIELD"),

    ("BaseType_t",  "tmrCOMMAND_DELETE"),
    ("BaseType_t",  "tmrCOMMAND_START"),
    ("BaseType_t",  "tmrCOMMAND_START_FROM_ISR"),
    ("BaseType_t",  "tmrCOMMAND_STOP"),
    ("BaseType_t",  "tmrCOMMAND_STOP_FROM_ISR"),
    ("BaseType_t",  "tmrCOMMAND_RESET"),
    ("BaseType_t",  "tmrCOMMAND_RESET_FROM_ISR"),
    ("BaseType_t",  "tmrCOMMAND_CHANGE_PERIOD"),
    ("BaseType_t",  "tmrCOMMAND_CHANGE_PERIOD_FROM_ISR"),

    ("BaseType_t",  "queueSEND_TO_BACK"),
    ("BaseType_t",  "queueSEND_TO_FRONT"),
    ("BaseType_t",  "queueOVERWRITE"),

    ("uint8_t",     "queueQUEUE_TYPE_BASE"),
    ("uint8_t",     "queueQUEUE_TYPE_BINARY_SEMAPHORE"),
    ("uint8_t",     "queueQUEUE_TYPE_MUTEX"),
    ("uint8_t",     "queueQUEUE_TYPE_RECURSIVE_MUTEX"),

    ("UBaseType_t", "semSEMAPHORE_QUEUE_ITEM_LENGTH"),
    ("TickType_t",  "semGIVE_BLOCK_TIME"),

    ("TickType_t",  "portMAX_DELAY"),
    ("TickType_t",  "portTICK_PERIOD_MS"),

    ("BaseType_t",  "taskSCHEDULER_SUSPENDED"),
    ("BaseType_t",  "taskSCHEDULER_NOT_STARTED"),
    ("BaseType_t",  "taskSCHEDULER_RUNNING"),
  ];

  for (ty, name) in constants {
    let name_undef = format!("__{name}_UNDEF__");

    writeln!(f, "#ifdef {name}")?;
    writeln!(f, "static const {ty} {name_undef} = {name};")?;
    writeln!(f, "#undef {name}")?;
    writeln!(f, "const {ty} {name} = {name_undef};")?;
    writeln!(f, "#endif")?;
  }

  Ok(())
}

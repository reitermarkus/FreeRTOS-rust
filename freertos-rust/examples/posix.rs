use core::time::Duration;

use freertos_rust::*;

#[global_allocator]
static GLOBAL: Allocator = Allocator;

fn main() {
  let x = Box::new(15);
  println!("Boxed int '{}' (allocator test)", x);

  println!("Starting FreeRTOS app ...");
  let hello_task = Task::new()
    .name("hello")
    .stack_size(128)
    .priority(TaskPriority::new(2).unwrap())
    .create(|task| for i in 0.. {
      println!("Hello from task! {}", i);
      task.delay(Duration::from_secs(1));
    });
  println!("Task {} started.", hello_task.name());
  //let free = freertos_rs_xPortGetFreeHeapSize();
  // println!("Free Memory: {}!", free);
  println!("Starting scheduler.");
  Scheduler::start();
}

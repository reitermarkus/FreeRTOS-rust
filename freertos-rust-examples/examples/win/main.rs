use freertos_rust::*;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;


fn main() {
    let x = Box::new(15);
    println!("Boxed int '{}' (allocator test)", x);

    set_assertion_handler(|_, _, _| { println!("Assert hook called") });

    //println!("Calling assert ...");
    //FreeRtosUtils::invoke_assert();

    println!("Starting FreeRTOS app ...");
    Task::new().name("hello").stack_size(128).priority(TaskPriority(2)).start(|_this_task| {
        let mut i = 0;
        loop {
            println!("Hello from Task! {}", i);
            CurrentTask::delay(Duration::ms(1000));
            i = i + 1;
        }
    }).unwrap();
    println!("Task registered");
    //let free = freertos_rs_xPortGetFreeHeapSize();
    // println!("Free Memory: {}!", free);
    println!("Starting scheduler");
    Scheduler::start();
}

#[test]
fn many_boxes() {
    init_allocator();
    println!("many_boxes... ");
    for i in 0..10 { // .. HEAP_SIZE
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    println!("[ok]");
}

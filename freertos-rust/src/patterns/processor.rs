use core::time::Duration;

use alloc2::{
  sync::{Arc, Weak},
  vec::Vec,
};

use crate::error::FreeRtosError;
use crate::sync::{Mutex, Queue};
use crate::ticks::Ticks;
use crate::interrupt_context::InterruptContext;

pub type SharedClientWithReplyQueue<O, const SIZE: usize> = Arc<ClientWithReplyQueue<O, SIZE>>;
pub type Client<I, const SIZE: usize> = ProcessorClient<I, (), SIZE>;
pub type ClientWithReplies<I, O, const SIZE: usize> = ProcessorClient<I, SharedClientWithReplyQueue<O, SIZE>, SIZE>;

pub trait ReplyableMessage {
    fn reply_to_client_id(&self) -> Option<usize>;
}

#[derive(Copy, Clone)]
pub struct InputMessage<I>
where
    I: Copy,
{
    val: I,
    reply_to_client_id: Option<usize>,
}

impl<I> InputMessage<I>
where
    I: Copy,
{
    pub fn request(val: I) -> Self {
        InputMessage {
            val: val,
            reply_to_client_id: None,
        }
    }

    pub fn request_with_reply(val: I, client_id: usize) -> Self {
        InputMessage {
            val: val,
            reply_to_client_id: Some(client_id),
        }
    }

    pub fn get_val(&self) -> I {
        self.val
    }
}

impl<I> ReplyableMessage for InputMessage<I>
where
    I: Copy,
{
    fn reply_to_client_id(&self) -> Option<usize> {
        self.reply_to_client_id
    }
}

pub struct Processor<I, O, const SIZE: usize>
where
    I: ReplyableMessage + Send + Copy,
    O: Send + Copy,
{
    queue: Arc<Queue<I, SIZE>>,
    inner: Arc<Mutex<ProcessorInner<O, SIZE>>>,
}

impl<I, O, const SIZE: usize> Processor<I, O, SIZE>
where
    I: ReplyableMessage + Send + Copy,
    O: Send + Copy,
{
    pub fn new() -> Result<Self, FreeRtosError> {
        let p = ProcessorInner {
            clients: Vec::new(),
            next_client_id: 1,
        };
        let p = Arc::new(Mutex::new(p));
        let p = Processor {
            queue: Arc::new(Queue::new()),
            inner: p,
        };
        Ok(p)
    }

    pub fn new_client(&self) -> Result<Client<I, SIZE>, FreeRtosError> {
        let c = ProcessorClient {
            processor_queue: Arc::downgrade(&self.queue),
            client_reply: (),
        };

        Ok(c)
    }

    pub fn new_client_with_reply(
        &self,
        timeout: impl Into<Ticks>,
    ) -> Result<ProcessorClient<I, SharedClientWithReplyQueue<O, SIZE>, SIZE>, FreeRtosError> {
        // TODO: Static assertion.
        if SIZE == 0 {
            return Err(FreeRtosError::InvalidQueueSize);
        }

        let client_reply = {
            let mut processor = self.inner.timed_lock(timeout)?;

            let c = ClientWithReplyQueue {
                id: processor.next_client_id,
                processor_inner: self.inner.clone(),
                receive_queue: Queue::new(),
            };

            let c = Arc::new(c);
            processor.clients.push((c.id, Arc::downgrade(&c)));

            processor.next_client_id += 1;

            c
        };

        let c = ProcessorClient {
            processor_queue: Arc::downgrade(&self.queue),
            client_reply: client_reply,
        };

        Ok(c)
    }

    pub fn get_receive_queue(&self) -> &Queue<I, SIZE> {
        &*self.queue
    }

    pub fn reply(
        &self,
        received_message: I,
        reply: O,
        timeout: impl Into<Ticks>,
    ) -> Result<bool, FreeRtosError> {
        if let Some(client_id) = received_message.reply_to_client_id() {
          let timeout = timeout.into();

            let inner = self.inner.timed_lock(timeout)?;
            if let Some(client) = inner
                .clients
                .iter()
                .flat_map(|ref x| x.1.upgrade().into_iter())
                .find(|x| x.id == client_id)
            {
                client.receive_queue.send(reply, timeout)?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl<I, O, const SIZE: usize> Processor<InputMessage<I>, O, SIZE>
where
    I: Send + Copy,
    O: Send + Copy,
{
    pub fn reply_val(
        &self,
        received_message: InputMessage<I>,
        reply: O,
        timeout: impl Into<Ticks>,
    ) -> Result<bool, FreeRtosError> {
        self.reply(received_message, reply, timeout)
    }
}

struct ProcessorInner<O, const SIZE: usize>
where
    O: Copy,
{
    clients: Vec<(usize, Weak<ClientWithReplyQueue<O, SIZE>>)>,
    next_client_id: usize,
}

impl<O, const SIZE: usize> ProcessorInner<O, SIZE>
where
    O: Copy,
{
    fn remove_client_reply(&mut self, client: &ClientWithReplyQueue<O, SIZE>) {
        self.clients.retain(|ref x| x.0 != client.id)
    }
}

pub struct ProcessorClient<I, C, const SIZE: usize>
where
    I: ReplyableMessage + Send + Copy,
{
    processor_queue: Weak<Queue<I, SIZE>>,
    client_reply: C,
}

impl<I, O, const SIZE: usize> ProcessorClient<I, O, SIZE>
where
    I: ReplyableMessage + Send + Copy,
{
    pub fn send(&self, message: I, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        let processor_queue = self
            .processor_queue
            .upgrade()
            .ok_or(FreeRtosError::ProcessorHasShutDown)?;
        processor_queue.send(message, timeout)?;
        Ok(())
    }

    pub fn send_from_isr(
        &self,
        context: &mut InterruptContext,
        message: I,
    ) -> Result<(), FreeRtosError> {
        let processor_queue = self
            .processor_queue
            .upgrade()
            .ok_or(FreeRtosError::ProcessorHasShutDown)?;
        processor_queue.send_from_isr(context, message)
    }
}

impl<I, const SIZE: usize> ProcessorClient<InputMessage<I>, (), SIZE>
where
    I: Send + Copy,
{
    pub fn send_val(&self, val: I, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        self.send(InputMessage::request(val), timeout)
    }

    pub fn send_val_from_isr(
        &self,
        context: &mut InterruptContext,
        val: I,
    ) -> Result<(), FreeRtosError> {
        self.send_from_isr(context, InputMessage::request(val))
    }
}

impl<I, O, const SIZE: usize> ProcessorClient<I, SharedClientWithReplyQueue<O, SIZE>, SIZE>
where
    I: ReplyableMessage + Send + Copy,
    O: Send + Copy,
{
    pub fn call(&self, message: I, timeout: impl Into<Ticks>) -> Result<O, FreeRtosError> {
      let timeout = timeout.into();

        self.send(message, timeout)?;
        self.client_reply.receive_queue.receive(timeout)
    }

    pub fn get_receive_queue(&self) -> &Queue<O, SIZE> {
        &self.client_reply.receive_queue
    }
}

impl<I, O, const SIZE: usize> ProcessorClient<InputMessage<I>, SharedClientWithReplyQueue<O, SIZE>, SIZE>
where
    I: Send + Copy,
    O: Send + Copy,
{
    pub fn send_val(&self, val: I, timeout: impl Into<Ticks>) -> Result<(), FreeRtosError> {
        self.send(InputMessage::request(val), timeout)
    }

    pub fn call_val(&self, val: I, timeout: impl Into<Ticks>) -> Result<O, FreeRtosError> {
        let reply = self.call(
            InputMessage::request_with_reply(val, self.client_reply.id),
            timeout,
        )?;
        Ok(reply)
    }
}

impl<I, C, const SIZE: usize> Clone for ProcessorClient<I, C, SIZE>
where
    I: ReplyableMessage + Send + Copy,
    C: Clone,
{
    fn clone(&self) -> Self {
        ProcessorClient {
            processor_queue: self.processor_queue.clone(),
            client_reply: self.client_reply.clone(),
        }
    }
}

pub struct ClientWithReplyQueue<O, const SIZE: usize>
where
    O: Copy,
{
    id: usize,
    processor_inner: Arc<Mutex<ProcessorInner<O, SIZE>>>,
    receive_queue: Queue<O, SIZE>,
}

impl<O, const SIZE: usize> Drop for ClientWithReplyQueue<O, SIZE>
where
    O: Copy,
{
    fn drop(&mut self) {
        if let Ok(mut p) = self.processor_inner.timed_lock(Duration::from_millis(1000)) {
            p.remove_client_reply(&self);
        }
    }
}

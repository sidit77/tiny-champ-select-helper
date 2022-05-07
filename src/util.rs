use async_std::sync::{Mutex, Arc};
use async_broadcast::{InactiveReceiver, Receiver};
use async_std::prelude::StreamExt;
use async_std::task;
use async_std::task::JoinHandle;

#[derive(Clone)]
pub struct ReceiveWrapper<T> {
    receiver: InactiveReceiver<T>,
    last_value: Arc<Mutex<T>>,
    _handle: Arc<JoinHandle<()>>
}

impl <T> ReceiveWrapper<T>
    where T: Default + Clone + Send + Sync + 'static
{
    pub fn new(receiver: Receiver<T>) -> Self {
        Self::new_with_default(receiver, T::default())
    }
}

impl<T> ReceiveWrapper<T>
    where T: Clone + Send + Sync + 'static
{

    pub fn new_with_default(receiver: Receiver<T>, default_value: impl Into<T>) -> Self {
        let last_value = Arc::new(Mutex::new(default_value.into()));
        let _handle = Arc::new({
            let last_value = last_value.clone();
            let mut receiver = receiver.clone();
            task::spawn(async move {
                while let Some(val) = receiver.next().await {
                    let mut x = last_value.lock_arc().await;
                    *x = val;
                }
            })
        });
        Self {
            receiver: receiver.deactivate(),
            last_value,
            _handle
        }
    }

    pub async fn subscribe(&self) -> (T, Receiver<T>) {
        let receiver = self.receiver.activate_cloned();
        let value = self.last_value.lock_arc().await.clone();
        (value, receiver)
    }

}
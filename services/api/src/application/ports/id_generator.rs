use uuid::Uuid;

pub trait IdGenerator: Send + Sync {
    fn new_uuid(&self) -> Uuid;
}

#[derive(Debug, Default)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn new_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct SequenceIdGenerator {
    ids: std::sync::Mutex<std::collections::VecDeque<Uuid>>,
}

#[cfg(test)]
impl SequenceIdGenerator {
    pub fn new(ids: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            ids: std::sync::Mutex::new(ids.into_iter().collect()),
        }
    }
}

#[cfg(test)]
impl IdGenerator for SequenceIdGenerator {
    fn new_uuid(&self) -> Uuid {
        self.ids
            .lock()
            .expect("sequence id generator mutex poisoned")
            .pop_front()
            .expect("test id sequence exhausted")
    }
}

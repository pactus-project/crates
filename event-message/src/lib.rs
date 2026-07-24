use erased_serde::Serialize;
use std::{any::Any, fmt::Debug};

pub trait EventMessage: Debug + Send + Sync + Serialize {
    fn key(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}

erased_serde::serialize_trait_object!(EventMessage);

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEvent {
        pub msg: String,
    }

    impl EventMessage for TestEvent {
        fn key(&self) -> String {
            "test-key".to_string()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_event_key() {
        let evt = TestEvent {
            msg: "hello_world".to_string(),
        };

        assert_eq!(evt.key(), "test-key".to_string());
    }

    #[test]
    fn test_event_boxed_serde_json() {
        let evt1 = TestEvent {
            msg: "hello_world".to_string(),
        };

        let boxed_event: Box<dyn EventMessage> = Box::new(evt1.clone());

        let mut buf = vec![];
        let mut serializer = serde_json::Serializer::new(&mut buf);
        boxed_event
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(&mut serializer))
            .unwrap();

        let mut deserializer = serde_json::Deserializer::from_slice(&buf);
        let evt2: TestEvent = erased_serde::deserialize(
            &mut <dyn erased_serde::Deserializer>::erase(&mut deserializer),
        )
        .unwrap();

        assert_eq!(evt1, evt2);
    }
}

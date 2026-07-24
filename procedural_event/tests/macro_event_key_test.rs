use event_message::EventMessage;
use procedural_event::Event;
use serde::{Deserialize, Serialize};

#[test]
fn test_event_key_1() {
    #[derive(Debug, Serialize, Deserialize, Event)]
    #[event_key("key")]
    pub struct Foo {}

    let f = Foo {};

    assert_eq!(f.key(), "key");
    assert_eq!(Foo::event_key, "key");
}

// TODO: uncomment me
// #[test]
// fn test_event_key_2() {
//     #[derive(Debug, Serialize, Deserialize, Event)]
//     #[event_key = "key"]
//     pub struct Foo {}

//     let f = Foo {};

//     assert_eq!(f.key(), "key");
//     assert_eq!(Foo::event_key, "key");
// }

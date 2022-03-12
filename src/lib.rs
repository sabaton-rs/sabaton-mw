use std::sync::{Arc, Mutex};
use cyclonedds_rs::{DdsParticipant, DdsPublisher, TopicType, PublisherBuilder, TopicBuilder, WriterBuilder, DdsWriter};
use error::MiddlewareError;
pub mod error;
/// Wrapper for Sabaton Middleware APIs

#[derive(Clone)]
pub struct InitContext;

impl InitContext {
    pub fn new() -> InitContext {
        InitContext {}
    }
}

pub struct Writer<T: TopicType>{
    writer : DdsWriter<T>
}

impl <T> Writer<T> where
T: TopicType, {
    pub fn publish(&mut self, msg:Arc<T>) -> Result<(),MiddlewareError> {
        self.writer.write(msg).map_err(|op| MiddlewareError::DDSError(op))
    }
}

#[derive(Clone)]
pub struct Node {
    inner : Arc<Mutex<NodeInner>>,
}

struct NodeInner {
    name : String,
    namespace: String,
    participant : DdsParticipant,
    maybe_publisher : Option<DdsPublisher>,
}

impl NodeInner {
    pub fn create(name : String, namespace: String) -> Result<NodeInner,MiddlewareError> {
        let participant = DdsParticipant::create(None, None, None)?;

        Ok(NodeInner {
            name,
            namespace,
            participant,
            maybe_publisher: None,
        })
    }
}


impl Node {

    pub fn create(context_ : InitContext, name : &str, namespace: &str) -> Result<Node,MiddlewareError> {
        let inner = NodeInner::create(name.into(), namespace.into())?;

        Ok(Node {
            inner : Arc::new(Mutex::new(inner)),
        })
    }

    pub fn advertise<T>(&mut self, topic_path: &str) -> Result<Writer<T>,MiddlewareError>
    where T : TopicType {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.maybe_publisher.is_none() {
                inner.maybe_publisher = Some(PublisherBuilder::new().create(&inner.participant)?);
            }
            assert!(!inner.maybe_publisher.is_none());

            let topic = TopicBuilder::<T>::new().with_name(topic_path.to_owned()).create(&inner.participant)?;
            let writer = WriterBuilder::new().create(inner.maybe_publisher.as_ref().unwrap(), topic)?;
            Ok(Writer{writer})
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

  
}


#[cfg(test)]
mod tests {
    use super::*;
    use cyclonedds_rs::Topic;
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use cyclonedds_rs::DdsQos;
    use cyclonedds_rs::DdsListener;
    use cyclonedds_rs::cdr;
    use cyclonedds_rs::SampleBuffer;
    use cyclonedds_rs::DdsTopic;
    use cyclonedds_rs::DDSError;
    
    #[test]
    fn it_works() {

        #[derive(Default, Deserialize, Serialize, Topic)]
        struct A {
            name : String,
        }

        let mut node = Node::create(InitContext::new(), "nodename", "namespace").expect("Node");

        let mut p = node.advertise::<A>("chatter").expect("cannot create writer");

        let a = A { name : "foo".to_owned()};

        p.publish(Arc::new(a)).expect("Cannot publish");
        
    }


}

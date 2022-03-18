/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Unauthorized copying of this file, via any medium is strictly prohibited
    Proprietary and confidential
    Sojan James <sojan.james@gmail.com>, 2021

    You are permitted to use this software as per the terms of the agreement
    between you and Sabaton Systems LLP.
*/

use cyclonedds_rs::{
    DdsParticipant, DdsPublisher, DdsReader, DdsSubscriber, DdsWriter, PublisherBuilder,
    ReaderBuilder, SampleBuffer, SubscriberBuilder, TopicBuilder, TopicType, WriterBuilder,
};
use error::MiddlewareError;
use someip::{ServerRequestHandler, CreateServerRequestHandler};
use std::sync::{Arc, Mutex};
pub mod error;
pub mod cdds;
/// Wrapper for Sabaton Middleware APIs
///

pub trait SyncReader<T: TopicType> {
    fn take_now(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError>;
}

pub trait AsyncReader<T: TopicType> {
    fn take(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError>;
}

#[derive(Clone)]
pub struct InitContext;

impl InitContext {
    pub fn new() -> InitContext {
        InitContext {}
    }
}

pub struct Writer<T: TopicType> {
    writer: DdsWriter<T>,
}

impl<T> Writer<T>
where
    T: TopicType,
{
    pub fn publish(&mut self, msg: Arc<T>) -> Result<(), MiddlewareError> {
        self.writer
            .write(msg)
            .map_err(|op| MiddlewareError::DDSError(op))
    }
}

pub struct Samples<T: TopicType> {
    samples: SampleBuffer<T>,
}

impl<T> Samples<T>
where
    T: TopicType,
{
    pub fn new(len: usize) -> Self {
        Self {
            samples: SampleBuffer::new(len),
        }
    }
}

pub struct Reader<T: TopicType> {
    reader: DdsReader<T>,
}

impl<T> SyncReader<T> for Reader<T>
where
    T: TopicType,
{
    fn take_now(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError> {
        self.reader
            .take_now(&mut samples.samples)
            .map_err(|e| e.into())
    }
}

#[derive(Clone)]
pub struct Node {
    inner: Arc<Mutex<NodeInner>>,
}

struct NodeInner {
    name: String,
    namespace: String,
    participant: DdsParticipant,
    maybe_publisher: Option<DdsPublisher>,
    maybe_subscriber: Option<DdsSubscriber>,
}

impl NodeInner {
    pub fn create(name: String, namespace: String) -> Result<NodeInner, MiddlewareError> {
        let participant = DdsParticipant::create(None, None, None)?;

        Ok(NodeInner {
            name,
            namespace,
            participant,
            maybe_publisher: None,
            maybe_subscriber: None,
        })
    }
}

impl Node {
    pub fn create(
        context_: InitContext,
        name: &str,
        namespace: &str,
    ) -> Result<Node, MiddlewareError> {
        let inner = NodeInner::create(name.into(), namespace.into())?;

        Ok(Node {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub fn advertise<T>(&mut self, topic_path: &str) -> Result<Writer<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.maybe_publisher.is_none() {
                inner.maybe_publisher = Some(PublisherBuilder::new().create(&inner.participant)?);
            }
            assert!(!inner.maybe_publisher.is_none());

            let topic = TopicBuilder::<T>::new()
                .with_name(topic_path.to_owned())
                .create(&inner.participant)?;
            let writer =
                WriterBuilder::new().create(inner.maybe_publisher.as_ref().unwrap(), topic)?;
            Ok(Writer { writer })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    pub fn subscribe<T>(&mut self, topic_path: &str) -> Result<impl SyncReader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.maybe_subscriber.is_none() {
                inner.maybe_subscriber = Some(SubscriberBuilder::new().create(&inner.participant)?);
            }
            assert!(!inner.maybe_subscriber.is_none());

            let topic = TopicBuilder::<T>::new()
                .with_name(topic_path.to_owned())
                .create(&inner.participant)?;
            let reader =
                ReaderBuilder::new().create(inner.maybe_subscriber.as_ref().unwrap(), topic)?;
            Ok(Reader { reader })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    pub fn subscribe_async<T>(&mut self, topic_path: &str) -> Result<Reader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.maybe_subscriber.is_none() {
                inner.maybe_subscriber = Some(SubscriberBuilder::new().create(&inner.participant)?);
            }
            assert!(!inner.maybe_subscriber.is_none());

            let topic = TopicBuilder::<T>::new()
                .with_name(topic_path.to_owned())
                .create(&inner.participant)?;
            let reader = ReaderBuilder::new()
                .as_async()
                .create(inner.maybe_subscriber.as_ref().unwrap(), topic)?;
            Ok(Reader { reader })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    //Hosting services
    pub fn serve<T:CreateServerRequestHandler<Item=T>>(&mut self, server_impl:Arc<T>) -> Result<(),MiddlewareError> {
        
        let handlers = T::create_server_request_handler(server_impl);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyclonedds_rs::cdr;
    use cyclonedds_rs::DDSError;
    use cyclonedds_rs::DdsListener;
    use cyclonedds_rs::DdsQos;
    use cyclonedds_rs::DdsTopic;
    use cyclonedds_rs::SampleBuffer;
    use cyclonedds_rs::Topic;
    use interface_example::EchoResponse;
    use interface_example::ExampleError;
    use interface_example::ExampleStatus;
    use interface_example::{Example, ExampleDispatcher};
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use someip::{ServerRequestHandlerEntry, ServiceIdentifier};
    use someip_derive::*;
    use async_trait::async_trait;
    #[test]
    fn it_works() {
        #[derive(Default, Deserialize, Serialize, Topic)]
        struct A {
            name: String,
        }

        let mut node = Node::create(InitContext::new(), "nodename", "namespace").expect("Node");

        let mut p = node
            .advertise::<A>("chatter")
            .expect("cannot create writer");

        let a = A {
            name: "foo".to_owned(),
        };

        p.publish(Arc::new(a)).expect("Cannot publish");
    }

    #[test]
    fn host_service() {
        #[service_impl(Example)]
        pub struct EchoServerImpl {}

        #[async_trait]
        impl Example for EchoServerImpl {
            
            async fn echo(&self, _data: String) -> Result<EchoResponse, ExampleError> {
                Err(ExampleError::Unknown)
            }

            fn set_status(
                &self,
                _status: interface_example::ExampleStatus,
            ) -> Result<(), someip::FieldError> {
                Ok(())
            }

            fn get_status(&self) -> Result<&interface_example::ExampleStatus, someip::FieldError> {
                Ok(&ExampleStatus::Ready)
            }
        }

        let mut node = Node::create(InitContext::new(), "nodename", "namespace").expect("Node");

        let server = Arc::new(EchoServerImpl {});

        node.serve(server);



    }
}

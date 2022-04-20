/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Sojan James <sojan.james@gmail.com>, 2021

    SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-sabaton-commercial
*/

use async_signals::Signals;
use async_trait::async_trait;
use cyclonedds_rs::{
    DdsParticipant, DdsPublisher, DdsReader, DdsSubscriber, DdsWriter, PublisherBuilder,
    ReaderBuilder, SampleBuffer, SubscriberBuilder, TopicBuilder, TopicType, WriterBuilder,
};
use error::MiddlewareError;
use futures::TryFutureExt;
use futures_util::StreamExt;
use qos::{QosDurability, QosHistory, QosReliability};
use services::get_service_ids;
use someip::{
    tasks::ConnectionInfo, Configuration, CreateServerRequestHandler, Proxy, ProxyConstruct,
    Server, ServerRequestHandler, ServerRequestHandlerEntry, ServiceIdentifier, ServiceVersion,
};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::runtime::Builder;
use tracing::{debug, error};

use crate::{
    cdds::service_discovery::{service_name_to_topic_name, ServiceInfo, Transport},
    config::get_bind_address,
    qos::{Qos, QosCreate},
    services::get_config_path,
};
pub mod cdds;
pub mod config;
pub mod error;
pub mod qos;
pub mod services;

pub use cdds::cdds::CddsQos as QosImpl;

const SERVICE_MAPPING_CONFIG_PATH: &str = "/etc/sabaton/services.toml";

pub trait SyncReader<T: TopicType> {
    fn take_now(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError>;
}

#[async_trait]
pub trait AsyncReader<T: TopicType> {
    async fn take(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError>;
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

    // Loan a buffer from the stack. This may not be supported always.
    // The topic must of Fixed size and shared memory must be enabled in
    // cyclone for this to work.
    pub fn loan(&mut self) -> Result<Loaned<T>, MiddlewareError> {
        match self.writer.loan() {
            Ok(l) => Ok(Loaned { inner: l }),
            Err(e) => match e {
                cyclonedds_rs::DDSError::NotEnabled => Err(MiddlewareError::SharedMemoryNotEnabled),
                _ => Err(MiddlewareError::DDSError(e)),
            },
        }
    }

    // Return the loan that was taken.  The buffer will be published if it is marked
    // as initialized by the ``Loaned::assume_init`` function. If not initialized
    // the buffer will be simple returned to the pool.
    pub fn return_loan(&mut self, buffer: Loaned<T>) -> Result<(), MiddlewareError> {
        self.writer
            .return_loan(buffer.inner)
            .map_err(|op| MiddlewareError::DDSError(op))
    }
}

pub struct Loaned<T: TopicType> {
    inner: cyclonedds_rs::dds_writer::Loaned<T>,
}

impl<T> Loaned<T>
where
    T: Sized + TopicType,
{
    pub fn as_mut_ptr(&mut self) -> Option<*mut T> {
        self.inner.as_mut_ptr()
    }

    pub fn assume_init(self) -> Self {
        Loaned {
            inner: self.inner.assume_init(),
        }
    }
}

pub struct SampleStorage<T: TopicType> {
    sample: cyclonedds_rs::serdes::SampleStorage<T>,
}

impl<T> Deref for SampleStorage<T>
where
    T: TopicType,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.sample.deref()
    }
}

pub struct Samples<T: TopicType> {
    samples: SampleBuffer<T>,
}

impl<'a, T> Samples<T>
where
    T: TopicType,
{
    pub fn new(len: usize) -> Self {
        Self {
            samples: SampleBuffer::new(len),
        }
    }

    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        let sample = self.samples.get(index).get();
        sample
    }

    pub fn iter(&self) -> impl Iterator<Item = SampleStorage<T>> + '_ {
        let p = self.samples.iter().filter_map(|p| match p.get_sample() {
            Some(s) => Some(SampleStorage { sample: s }),
            None => None,
        });
        p
    }
}

unsafe impl<T> Sync for Reader<T> where T: TopicType {}

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

#[async_trait]
impl<T> AsyncReader<T> for Reader<T>
where
    T: TopicType + std::marker::Send + std::marker::Sync,
{
    async fn take(&mut self, samples: &mut Samples<T>) -> Result<usize, MiddlewareError> {
        let res = self
            .reader
            .take(&mut samples.samples)
            .err_into::<MiddlewareError>()
            .await;

        res
    }
}

#[derive(Clone)]
pub struct Node {
    inner: Arc<RwLock<NodeInner>>,
}

pub struct NodeBuilder {
    group: String,
    instance: String,
    num_workers: usize,
    single_threaded: bool,
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self {
            group: "default".to_owned(),
            instance: "0".to_owned(),
            num_workers: 1,
            single_threaded: true,
        }
    }
}

impl NodeBuilder {
    pub fn with_group(mut self, group: String) -> Self {
        self.group = group;
        self
    }

    pub fn with_instance(mut self, instance: String) -> Self {
        self.instance = instance;
        self
    }

    pub fn with_group_and_instance(mut self, group: String, instance: String) -> Self {
        self.group = group;
        self.instance = instance;

        self
    }

    pub fn multi_threaded(mut self) -> Self {
        self.single_threaded = false;
        self
    }

    pub fn with_num_workers(mut self, num_workers: usize) -> Self {
        if !self.single_threaded {
            self.num_workers = num_workers;
            self
        } else {
            panic!("workers not allowed on single threaded runtime");
        }
    }

    pub fn build(self, name: String) -> Result<Node, MiddlewareError> {
        let participant = DdsParticipant::create(None, None, None)?;

        let inner = NodeInner {
            name,
            group: self.group,
            instance: self.instance,
            participant,
            maybe_publisher: None,
            maybe_subscriber: None,
            handlers: Vec::new(),
            next_client_id: 0,
            proxies: Vec::new(),
            single_threaded: self.single_threaded,
            num_workers: self.num_workers,
        };

        Ok(Node {
            inner: Arc::new(RwLock::new(inner)),
        })
    }
}

struct NodeInner {
    name: String,
    group: String,
    instance: String,
    participant: DdsParticipant,
    maybe_publisher: Option<DdsPublisher>,
    maybe_subscriber: Option<DdsSubscriber>,
    handlers: Vec<ServerRequestHandlerEntry>,
    next_client_id: u16,
    proxies: Vec<(String, Box<dyn Proxy>, u8, u32)>,
    single_threaded: bool,
    num_workers: usize,
}

#[derive(Default)]
pub struct PublishOptions {
    group: Option<String>,
    instance: Option<String>,
    reliability: Option<QosReliability>,
    durability: Option<QosDurability>,
    history: Option<QosHistory>,
}

impl PublishOptions {
    pub fn with_group(&mut self, group: &str) -> &mut Self {
        let _ = self.group.replace(group.to_owned());
        self
    }

    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        let _ = self.instance.replace(instance.to_owned());
        self
    }

    pub fn with_reliability(&mut self, reliability: QosReliability) -> &mut Self {
        let _ = self.reliability.replace(reliability);
        self
    }

    pub fn with_durability(&mut self, durability: QosDurability) -> &mut Self {
        let _ = self.durability.replace(durability);
        self
    }

    pub fn with_history(&mut self, history: QosHistory) -> &mut Self {
        let _ = self.history.replace(history);
        self
    }
}

#[derive(Default)]
pub struct SubscribeOptions {
    group: Option<String>,
    instance: Option<String>,
    reliability: Option<QosReliability>,
    durability: Option<QosDurability>,
    history: Option<QosHistory>,
}

impl SubscribeOptions {
    pub fn with_group(&mut self, group: &str) -> &mut Self {
        let _ = self.group.replace(group.to_owned());
        self
    }

    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        let _ = self.instance.replace(instance.to_owned());
        self
    }

    pub fn with_reliability(&mut self, reliability: QosReliability) -> &mut Self {
        let _ = self.reliability.replace(reliability);
        self
    }

    pub fn with_durability(&mut self, durability: QosDurability) -> &mut Self {
        let _ = self.durability.replace(durability);
        self
    }

    pub fn with_history(&mut self, history: QosHistory) -> &mut Self {
        let _ = self.history.replace(history);
        self
    }
}

impl Node {
    fn get_topic_prefix(group: &str, instance: &str) -> Option<String> {
        let prefix = format!("/{}/{}", group, instance);
        Some(prefix)
    }

    fn advertise_internal<T>(&self, topic_path: &str) -> Result<Writer<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
            if inner.maybe_publisher.is_none() {
                inner.maybe_publisher = Some(PublisherBuilder::new().create(&inner.participant)?);
            }
            assert!(inner.maybe_publisher.is_some());

            let topic = TopicBuilder::<T>::new()
                .with_name(topic_path.to_owned())
                .create(&inner.participant)?;

            // Use the default Qos
            let qos = QosImpl::default();

            let writer = WriterBuilder::new()
                .with_qos(qos.into())
                .create(inner.maybe_publisher.as_ref().unwrap(), topic)?;
            Ok(Writer { writer })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    pub fn advertise<T>(&self, options: &PublishOptions) -> Result<Writer<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
            if inner.maybe_publisher.is_none() {
                inner.maybe_publisher = Some(PublisherBuilder::new().create(&inner.participant)?);
            }
            assert!(inner.maybe_publisher.is_some());

            let topic_builder = TopicBuilder::<T>::new();

            let group = if let Some(group) = options.group.as_ref() {
                group
            } else {
                &inner.group
            };

            let instance = if let Some(instance) = options.instance.as_ref() {
                instance
            } else {
                &inner.instance
            };

            let topic_builder = if let Some(prefix) = Self::get_topic_prefix(&group, &instance) {
                topic_builder.with_name_prefix(prefix)
            } else {
                topic_builder
            };

            let topic = topic_builder.create(&inner.participant)?;

            let mut qos = QosImpl::create();
            if options.durability.is_some() {
                qos.set_durability(options.durability.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_durability(QosDurability::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }
            if options.reliability.is_some() {
                qos.set_reliability(options.reliability.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_reliability(QosReliability::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }
            if options.history.is_some() {
                qos.set_history(options.history.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_history(QosHistory::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }

            let writer = WriterBuilder::new()
                .with_qos(qos.into())
                .create(inner.maybe_publisher.as_ref().unwrap(), topic)?;
            Ok(Writer { writer })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    pub fn subscribe<T>(
        &self,
        options: &SubscribeOptions,
    ) -> Result<impl SyncReader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
            if inner.maybe_subscriber.is_none() {
                inner.maybe_subscriber = Some(SubscriberBuilder::new().create(&inner.participant)?);
            }
            assert!(inner.maybe_subscriber.is_some());

            let group = if let Some(group) = options.group.as_ref() {
                group
            } else {
                &inner.group
            };

            let instance = if let Some(instance) = options.instance.as_ref() {
                instance
            } else {
                &inner.instance
            };

            let topic_builder = TopicBuilder::<T>::new();

            let topic_builder = if let Some(prefix) = Self::get_topic_prefix(&group, &instance) {
                topic_builder.with_name_prefix(prefix)
            } else {
                topic_builder
            };

            let topic = topic_builder.create(&inner.participant)?;

            let mut qos = QosImpl::create();
            if options.durability.is_some() {
                qos.set_durability(options.durability.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_durability(QosDurability::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }
            if options.reliability.is_some() {
                qos.set_reliability(options.reliability.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_reliability(QosReliability::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }
            if options.history.is_some() {
                qos.set_history(options.history.as_ref().unwrap().clone())
                    .map_err(|_e| MiddlewareError::QosError)?;
            } else {
                qos.set_history(QosHistory::default())
                    .map_err(|_e| MiddlewareError::QosError)?;
            }

            let reader = ReaderBuilder::new()
                .with_qos(qos.into())
                .create(inner.maybe_subscriber.as_ref().unwrap(), topic)?;
            Ok(Reader { reader })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    fn subscribe_async_internal<T>(&self, topic_path: &str) -> Result<Reader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
            if inner.maybe_subscriber.is_none() {
                inner.maybe_subscriber = Some(SubscriberBuilder::new().create(&inner.participant)?);
            }
            assert!(inner.maybe_subscriber.is_some());

            let topic = TopicBuilder::<T>::new()
                .with_name(topic_path.to_owned())
                .create(&inner.participant)?;

            let qos = QosImpl::default();
            let reader = ReaderBuilder::new()
                .with_qos(qos.into())
                .as_async()
                .create(inner.maybe_subscriber.as_ref().unwrap(), topic)?;
            Ok(Reader { reader })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    pub fn subscribe_async<T>(&self) -> Result<Reader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
            if inner.maybe_subscriber.is_none() {
                inner.maybe_subscriber = Some(SubscriberBuilder::new().create(&inner.participant)?);
            }
            assert!(inner.maybe_subscriber.is_some());

            let topic_builder = TopicBuilder::<T>::new();

            let topic_builder =
                if let Some(prefix) = Self::get_topic_prefix(&inner.group, &inner.instance) {
                    topic_builder.with_name_prefix(prefix)
                } else {
                    topic_builder
                };

            let topic = topic_builder.create(&inner.participant)?;

            let qos = QosImpl::default();
            let reader = ReaderBuilder::new()
                .with_qos(qos.into())
                .as_async()
                .create(inner.maybe_subscriber.as_ref().unwrap(), topic)?;
            Ok(Reader { reader })
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    // create a proxy for a service
    pub fn create_proxy<
        T: 'static + Proxy + ProxyConstruct + ServiceIdentifier + ServiceVersion + Clone,
    >(
        &self,
    ) -> Result<T, MiddlewareError> {
        let config = Arc::new(Configuration::default());
        let config_path = get_config_path()?;
        let service_ids = vec![T::service_name()];
        let service_ids = get_service_ids(&config_path, &service_ids)?;
        if service_ids.len() != 1 {
            return Err(MiddlewareError::ConfigurationError);
        }
        if let Ok(mut inner) = self.inner.write() {
            let proxy = T::new(service_ids[0].1, inner.next_client_id, config);
            inner.next_client_id += 1;
            inner.proxies.push((
                T::service_name().to_owned(),
                Box::new(proxy.clone()),
                T::__major_version__(),
                T::__minor_version__(),
            ));
            Ok(proxy)
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    //Hosting services
    pub fn serve<T: CreateServerRequestHandler<Item = T>>(
        &self,
        server_impl: Arc<T>,
    ) -> Result<(), MiddlewareError> {
        let mut handlers = T::create_server_request_handler(server_impl);
        if let Ok(mut inner) = self.inner.write() {
            inner.handlers.append(&mut handlers);
            Ok(())
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    }

    /// The main processing loop of the node.  This function will block waiting for events and pumping
    /// the necessary callbacks.
    pub fn spin<F>(&self, main_function: F) -> Result<(), MiddlewareError>
    where
        F: 'static + Send + FnOnce(),
    {
        let mut builder = if self.inner.read().unwrap().single_threaded {
            Builder::new_current_thread()
        } else {
            let mut builder = Builder::new_multi_thread();
            builder.worker_threads(self.inner.read().unwrap().num_workers);
            builder
        };

        builder.thread_name(format!("{}-worker", self.inner.read().unwrap().name));
        builder.enable_all();

        let rt = builder
            .build()
            .map_err(|_e| MiddlewareError::InternalError)?;

        let config = Arc::new(Configuration::default());

        let maybe_services = if let Ok(inner) = self.inner.read() {
            let service_handlers: Vec<&str> = inner.handlers.iter().map(|h| h.name).collect();

            let maybe_services = if !service_handlers.is_empty() {
                let config_path = get_config_path()?;
                let services = crate::services::get_service_ids(&config_path, &service_handlers)?;

                if services.len() != service_handlers.len() {
                    error!("Could not get all service IDs");
                    return Err(MiddlewareError::ConfigurationError);
                }

                let services: Vec<(String, u16, u8, u32, u16, Arc<dyn ServerRequestHandler>)> =
                    services
                        .into_iter()
                        .map(|(s, id)| {
                            let h = inner
                                .handlers
                                .iter()
                                .find(|h| h.name == s.as_str())
                                .unwrap();

                            (
                                s,
                                id,
                                h.major_version,
                                h.minor_version,
                                h.instance_id,
                                h.handler.clone(),
                            )
                        })
                        .collect();

                Some(services)
            } else {
                None
            };
            maybe_services
        } else {
            None
        };

        let maybe_proxies = if let Ok(mut inner) = self.inner.write() {
            let proxies: Vec<(String, Box<dyn Proxy>, u8, u32)> =
                inner.proxies.drain(0..).collect();
            Some(proxies)
        } else {
            None
        };

        debug!("starting tokio main loop");
        // blocking main loop
        let _result = rt.block_on(async {
            if let Some(services) = maybe_services {
                for (service, service_id, major_version, minor_version, instance_id, handler) in
                    services
                {
                    let config = config.clone();
                    let node_name = self.inner.read().unwrap().name.clone();
                    let topic_name = service_name_to_topic_name(&service);
                    let mut sd_publisher = self
                        .advertise_internal::<ServiceInfo>(&topic_name)
                        .expect("Unable to create topic publisher for SD");

                    let (tx, mut rx) = Server::create_notify_channel(2);

                    // Tokio task to publish Service discovery topic for this service
                    tokio::spawn(async move {
                        loop {
                            if let Some(msg) = rx.recv().await {
                                match msg {
                                    ConnectionInfo::ConnectionDropped(_i) => {}
                                    ConnectionInfo::UdpServerSocket(s) => {
                                        println!("Local UDP socket {:?}", s);
                                        let service_info = ServiceInfo {
                                            node: node_name.clone(),
                                            major_version,
                                            minor_version,
                                            instance_id,
                                            socket_address: s,
                                            transport: Transport::Udp,
                                            service_id,
                                        };
                                        println!("Going to Publish SD packet for Udp");

                                        sd_publisher
                                            .publish(Arc::new(service_info))
                                            .expect("Unable to publish SD topic");
                                        println!("Published SD packet");
                                    }
                                    ConnectionInfo::TcpServerSocket(s) => {
                                        println!("Local TCP socket {:?}", s);

                                        let service_info = ServiceInfo {
                                            node: node_name.clone(),
                                            major_version,
                                            minor_version,
                                            instance_id,
                                            socket_address: s,
                                            transport: Transport::Tcp,
                                            service_id,
                                        };
                                        println!("Going to Publish SD packet");

                                        sd_publisher
                                            .publish(Arc::new(service_info))
                                            .expect("Unable to publish SD topic");
                                        println!("Published SD packet");
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });

                    // tokio task for this service
                    tokio::spawn(async move {
                        //let test_service : Box<dyn ServerRequestHandler + Send> = Box::new(EchoServerImpl::default());
                        //let handler = EchoServerImpl::create_server_request_handler(Arc::new(EchoServerImpl::default()));
                        debug!("Going to run server for {}", &service);
                        println!(
                            "Going to run server for {} at address {:?}",
                            &service,
                            get_bind_address()
                        );
                        let res = Server::serve(
                            get_bind_address(),
                            handler.clone(),
                            config,
                            service_id,
                            major_version,
                            minor_version,
                            tx,
                        )
                        .await;
                        println!("Server terminated");
                        if let Err(e) = res {
                            println!("Server error:{}", e);
                        }
                    });
                }
            }

            let instance_id = 0;
            // launch the proxies
            if let Some(proxies) = maybe_proxies {
                for (name, proxy, major_version, minor_version) in proxies {
                    let topic_name = service_name_to_topic_name(&name);
                    println!("Starting proxy for {} at {}", &name, &topic_name);
                    let mut sd_subsriber = self
                        .subscribe_async_internal::<ServiceInfo>(&topic_name)
                        .unwrap();

                    // max of 5 instances for a services. TODO: this could be in a config file
                    let mut samples = Samples::<ServiceInfo>::new(5);
                    let client = proxy.get_dispatcher();

                    tokio::spawn(async move {
                        let mut is_running = false;
                        loop {
                            if let Ok(_len) = sd_subsriber.take(&mut samples).await {
                                for sample in samples.iter() {
                                    println!("Got sample {:?}", sample.deref());

                                    let sample_socket_address = sample.socket_address;

                                    if sample.major_version == major_version
                                        && sample.instance_id == instance_id
                                        && sample.minor_version == minor_version
                                        && !is_running
                                        && sample.transport == Transport::Tcp
                                    {
                                        let name = name.clone();
                                        let client = client.clone();
                                        tokio::spawn(async move {
                                            debug!(
                                                "Going to run proxy for {} connecting to {}",
                                                &name, sample_socket_address
                                            );
                                            if let Err(_res) =
                                                client.run(sample_socket_address).await
                                            {
                                                error!("Proxy run returned error");
                                            }
                                        });
                                        is_running = true;
                                    } else {
                                        // ignore
                                    }
                                }
                                tokio::time::sleep(Duration::from_millis(1000)).await;
                            }
                        }
                    });
                }
            }

            // launch the main function

            tokio::spawn(async move {
                main_function();
            });

            // wait for SIGINT
            let mut signals = Signals::new(vec![libc::SIGINT]).unwrap();
            let _signal = signals.next().await.unwrap();
            debug!("SIGINT received");
        });
        debug!("Tokio main loop exited");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;
    use async_trait::async_trait;
    use cyclonedds_rs::cdr;
    use cyclonedds_rs::DDSError;
    use cyclonedds_rs::DdsListener;
    use cyclonedds_rs::DdsQos;
    use cyclonedds_rs::DdsTopic;
    use cyclonedds_rs::SampleBuffer;
    use cdds_derive::Topic;
    use interface_example::EchoResponse;
    use interface_example::ExampleError;
    use interface_example::ExampleProxy;
    use interface_example::ExampleStatus;
    use interface_example::{Example, ExampleDispatcher};
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use someip::CallProperties;
    use someip::{ServerRequestHandlerEntry, ServiceIdentifier, ServiceInstance, ServiceVersion};
    use someip_derive::*;
    #[test]
    fn it_works() {
        #[derive(Default, Deserialize, Serialize, Topic)]
        struct A {
            name: String,
        }

        let node = NodeBuilder::default()
            .with_group_and_instance("group_name".to_owned(), "instance_name".to_owned())
            .build("nodename".to_owned())
            .expect("Node creation");
        let mut subscriber = node
            .subscribe::<A>(&SubscribeOptions::default())
            .expect("unable ti subscribe");

        let mut p = node
            .advertise::<A>(&PublishOptions::default())
            .expect("cannot create writer");

        let a = A {
            name: "foo".to_owned(),
        };

        p.publish(Arc::new(a)).expect("Cannot publish");

        let mut samples = Samples::<A>::new(2);
        subscriber.take_now(&mut samples).expect("Unable to take");
        for i in 0..2 {
            if let Some(a) = samples.get(i) {
                println!("A -> {}", a.name);
            }
        }
    }

    #[test]
    fn host_service() {
        std::env::set_var(SERVICE_MAPPING_CONFIG_PATH, "services.toml");
        #[service_impl(Example)]
        pub struct EchoServerImpl {}

        impl ServiceInstance for EchoServerImpl {}
        impl ServiceVersion for EchoServerImpl {}

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

        let node = NodeBuilder::default()
            .build("nodename".to_owned())
            .expect("Node creation");

        let server = Arc::new(EchoServerImpl {});

        node.serve(server).expect("Unable to serve");

        node.spin(|| println!("This is the main function"))
            .expect("Unable to spin");
    }

    #[test]
    fn client() {
        std::env::set_var(SERVICE_MAPPING_CONFIG_PATH, "services.toml");
        tracing_subscriber::fmt::init();

        // Client node in separate thread

        thread::spawn(|| {
            let client_node = NodeBuilder::default()
                .build("client".to_owned())
                .expect("Node creation");
            let proxy = client_node
                .create_proxy::<ExampleProxy>()
                .expect("Unable to create proxy");

            client_node
                .spin(move || {
                    let proxy = proxy.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(3000)).await;

                        let call_properties =
                            CallProperties::with_timeout(Duration::from_millis(5000));

                        match proxy.echo("Hello".to_string(), &call_properties).await {
                            Ok(res) => {
                                assert_eq!(res.echo.as_str(), "Hello");
                                println!("Received echo");
                            }
                            Err(e) => {
                                println!("Error:{:?}", e);
                                panic!("Echo response failed");
                            }
                        }
                    });
                })
                .expect("Unable to spin");
            //let cloned = node.clone();
        });

        #[service_impl(Example)]
        pub struct EchoServerImpl {}

        impl ServiceInstance for EchoServerImpl {}
        impl ServiceVersion for EchoServerImpl {}

        #[async_trait]
        impl Example for EchoServerImpl {
            async fn echo(&self, data: String) -> Result<EchoResponse, ExampleError> {
                let response = EchoResponse { echo: data };
                Ok(response)
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

        // Server node
        let node = NodeBuilder::default()
            .build("server".to_owned())
            .expect("Node creation");

        let server = Arc::new(EchoServerImpl {});

        node.serve(server).expect("Unable to serve");

        node.spin(|| {
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_millis(100));
                loop {
                    let _ = ticker.tick().await;
                }
            });
        })
        .expect("Unable to spin");
    }
}

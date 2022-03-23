/*
    Copyright (C) Sabaton Systems LLP - All Rights Reserved
    Unauthorized copying of this file, via any medium is strictly prohibited
    Proprietary and confidential
    Sojan James <sojan.james@gmail.com>, 2021

    You are permitted to use this software as per the terms of the agreement
    between you and Sabaton Systems LLP.
*/

use async_std::{path::PathBuf};
use async_trait::async_trait;
use cyclonedds_rs::{
    DdsParticipant, DdsPublisher, DdsReader, DdsSubscriber, DdsWriter, PublisherBuilder,
    ReaderBuilder, SampleBuffer, SubscriberBuilder, TopicBuilder, TopicType, WriterBuilder, error::ReaderError,
};
use error::MiddlewareError;
use futures::TryFutureExt;
use services::get_service_ids;
use someip::{ServerRequestHandler, CreateServerRequestHandler, ServerRequestHandlerEntry, Server, tasks::ConnectionInfo, Configuration, Proxy, ProxyConstruct,ServiceIdentifier, ServiceVersion};
use tracing::{debug, error};
use std::{sync::{Arc, Mutex, RwLock}, path::Path, time::Duration};
use tokio::{runtime::Runtime, runtime::Builder};
use async_signals::Signals;
use futures_util::StreamExt;

use fix_hidden_lifetime_bug::fix_hidden_lifetime_bug;

use crate::{services::get_config_path, cdds::service_discovery::{service_name_to_topic_name, ServiceInfo, Transport}, config::get_bind_address};
pub mod error;
pub mod cdds;
pub mod services;
pub mod config;

const SERVICE_MAPPING_CONFIG_PATH : &str = "/etc/sabaton/services.toml";

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
}

pub struct Sample<T:TopicType> {
    sample : cyclonedds_rs::Sample<T>
}

impl <T> Sample <T>
where 
    T: TopicType
{

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

    pub fn get(&self, index:usize) -> Option<Arc<T>> {
        let sample = self.samples.get(index).get();
        sample
    }
 
}


unsafe impl <T> Sync for Reader<T> where
T: TopicType, {}

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
        let res = self.reader.take(&mut samples.samples)
            .err_into::<MiddlewareError>().await;

        res
    
    }
    
}

#[derive(Clone)]
pub struct Node {
    inner: Arc<RwLock<NodeInner>>,
}

pub struct NodeBuilder {
    namespace : String,
    num_workers : usize,
    single_threaded : bool,
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self { namespace: Default::default(), num_workers: 1, single_threaded: true }
    }
}

impl NodeBuilder {
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
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

    pub fn build(mut self, name: String) -> Result<Node, MiddlewareError> {
        let participant = DdsParticipant::create(None, None, None)?;

        let inner = NodeInner {
            name,
            namespace: self.namespace,
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
    namespace: String,
    participant: DdsParticipant,
    maybe_publisher: Option<DdsPublisher>,
    maybe_subscriber: Option<DdsSubscriber>,
    handlers : Vec<ServerRequestHandlerEntry>,
    next_client_id : u16,
    proxies : Vec<(String, Box<dyn Proxy>, u8, u32)>,
    single_threaded : bool,
    num_workers : usize,
}


impl Node {

    pub fn advertise<T>(&self, topic_path: &str) -> Result<Writer<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
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

    pub fn subscribe<T>(&self, topic_path: &str) -> Result<impl SyncReader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
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

    pub fn subscribe_async<T>(&self, topic_path: &str) -> Result<Reader<T>, MiddlewareError>
    where
        T: TopicType,
    {
        if let Ok(mut inner) = self.inner.write() {
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

    // create a proxy for a service
    pub fn create_proxy<T: 'static + Proxy + ProxyConstruct + ServiceIdentifier + ServiceVersion + Clone>(&self) -> Result<T,MiddlewareError> {
        let config = Arc::new(Configuration::default());
        let config_path = get_config_path()?;
        let service_ids = vec![T::service_name()];
        let service_ids = get_service_ids(&config_path, &service_ids)?;
        if service_ids.len() != 1 {
            return Err(MiddlewareError::ConfigurationError)
        }
        if let Ok(mut inner) = self.inner.write() { 
            let proxy = T::new(service_ids[0].1, inner.next_client_id, config);
            inner.next_client_id += 1;
            inner.proxies.push( (T::service_name().to_owned(), Box::new(proxy.clone()),T::__major_version__(),T::__minor_version__()));
            Ok(proxy)
        } else {
            Err(MiddlewareError::InconsistentDataStructure)
        }
    } 

    //Hosting services
    pub fn serve<T:CreateServerRequestHandler<Item=T>>(&self, server_impl:Arc<T>) -> Result<(),MiddlewareError> {
        
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
    pub fn spin<F>(&self, mut main_function : F )  -> Result<(),MiddlewareError>
    where F: 'static + Send + FnMut() -> ()
    {
        let mut builder = if self.inner.read().unwrap().single_threaded {
           let builder =  Builder::new_current_thread();
           builder
        } else {
            let mut builder = Builder::new_multi_thread();
            builder.worker_threads(self.inner.read().unwrap().num_workers);
            builder
        };

        builder.thread_name(format!("{}-worker",self.inner.read().unwrap().name));
        builder.enable_all();

        let rt = builder.build().map_err(|_e| MiddlewareError::InternalError)?;

        let config = Arc::new(Configuration::default());

        let maybe_services = if let Ok(inner) = self.inner.read() {
        
            let service_handlers : Vec<&str> = inner.handlers.iter().map(|h| {
                h.name
            }).collect();

            let maybe_services = if service_handlers.len() > 0 {
                let config_path = get_config_path()?;
                let services = crate::services::get_service_ids(&config_path, &service_handlers)?;

                if services.len() != service_handlers.len() {
                    error!("Could not get all service IDs");
                    return Err(MiddlewareError::ConfigurationError)
                }

                let services : Vec<(String,u16,u8,u32,u16,Arc<dyn ServerRequestHandler>)> = services.into_iter().map(|(s,id)| {
                    let h = inner.handlers.iter().find(|h| h.name == s.as_str()).unwrap();

                    (s,id,h.major_version,h.minor_version, h.instance_id, h.handler.clone()) 
                }).collect();

                Some(services)
            } else {
                None
            };
            maybe_services

        } else { None};

        let mut maybe_proxies = if let Ok(mut inner) = self.inner.write() { 
            let proxies : Vec<(String, Box<dyn Proxy>,u8,u32)> = inner.proxies.drain(0..).collect();
            Some(proxies)
        } else {
            None
        };

            debug!("starting tokio main loop");
            // blocking main loop
            let _result = rt.block_on(async { 

                if let Some(services) = maybe_services {
                    for (service, service_id, major_version, minor_version, instance_id, handler) in services {
                        let config = config.clone();
                        let node_name = self.inner.read().unwrap().name.clone();
                        let topic_name = service_name_to_topic_name(&service);
                        let mut sd_publisher = self.advertise::<ServiceInfo>(&topic_name).expect("Unable to create topic publisher for SD");

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
                                                service_id
                                            };
                                            println!("Going to Publish SD packet for Udp");

                                            sd_publisher.publish(Arc::new(service_info)).expect("Unable to publish SD topic");
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
                                                service_id
                                            };
                                            println!("Going to Publish SD packet");

                                            sd_publisher.publish(Arc::new(service_info)).expect("Unable to publish SD topic");
                                            println!("Published SD packet");
                                        },
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
                            println!("Going to run server for {} at address {:?}", &service, get_bind_address());
                            let res = Server::serve(get_bind_address(), handler.clone(), config, service_id,major_version,minor_version, tx).await;
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
                       let mut sd_subsriber = self.subscribe_async::<ServiceInfo>(&topic_name).unwrap();
                   
                       // max of 5 instances for a services. TODO: this could be in a config file
                       let mut samples = Samples::<ServiceInfo>::new(5);
                       let client = proxy.get_dispatcher();

                       tokio::spawn( async move {
                        let mut is_running = false;
                        loop {
                            
                            if let Ok(len) = sd_subsriber.take(&mut samples).await {
                                for i in 0..len {
                                    if let Some(sample) = samples.get(i) {
                                        println!("Got sample {:?}", sample);
                                        if sample.major_version == major_version && sample.instance_id == instance_id &&
                                            sample.minor_version == minor_version && !is_running  && sample.transport == Transport::Tcp {
                                            
                                            let name = name.clone();
                                            let client = client.clone();
                                            tokio::spawn(async move {
                                                debug!("Going to run proxy for {} connecting to {}", &name, sample.socket_address );
                                                if let Err(res) = client.run(sample.socket_address).await {
                                                    error!("Proxy run returned error");
                                                } 
                                            });
                                            is_running = true;
            
                                            } else {
                                                // ignore
                                            }
                                    }
                                }
                                tokio::time::sleep(Duration::from_millis(1000)).await;                
                            }
                        }
                    });
                   

                    }
                }

                // launch the main function

                tokio::spawn( async move {
                    main_function();
                    }
                );

                // wait for SIGINT
                let mut signals = Signals::new(vec![libc::SIGINT]).unwrap();
                loop {
                    let signal = signals.next().await.unwrap();
                    debug!("SIGINT received");
                    break;
                }
            });
            debug!("Tokio main loop exited");
        Ok(())

    }

}



#[cfg(test)]
mod tests {
    use std::thread;

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
    use interface_example::ExampleProxy;
    use interface_example::ExampleStatus;
    use interface_example::{Example, ExampleDispatcher};
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use someip::CallProperties;
    use someip::{ServerRequestHandlerEntry, ServiceIdentifier, ServiceInstance, ServiceVersion};
    use someip_derive::*;
    use async_trait::async_trait;
    #[test]
    fn it_works() {
        #[derive(Default, Deserialize, Serialize, Topic)]
        struct A {
            name: String,
        }

        let mut node =   NodeBuilder::default().build("nodename".to_owned()).expect("Node creation");
        let mut subscriber = node.subscribe::<A>("chatter").expect("unable ti subscribe");

        let mut p = node
            .advertise::<A>("chatter")
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

        let mut node =   NodeBuilder::default().build("nodename".to_owned()).expect("Node creation");

        let server = Arc::new(EchoServerImpl {});

        node.serve(server).expect("Unable to serve");

        node.spin(|| {

            println!("This is the main function")

        }).expect("Unable to spin");

    }


#[test]
fn client() {
    std::env::set_var(SERVICE_MAPPING_CONFIG_PATH, "services.toml");
    tracing_subscriber::fmt::init();
    
    // Client node in separate thread

    thread::spawn( || {
    let mut client_node = NodeBuilder::default().build("client".to_owned()).expect("Node creation");
    let proxy = client_node.create_proxy::<ExampleProxy>().expect("Unable to create proxy");
    
    client_node.spin( move ||  {

        let proxy = proxy.clone();
        tokio::spawn( async move {

            tokio::time::sleep(Duration::from_millis(3000)).await;

            let call_properties = CallProperties::with_timeout(Duration::from_millis(5000));

            match proxy.echo("Hello".to_string(),&call_properties).await {
                Ok(res) => {assert_eq!(res.echo.as_str(), "Hello" ); println!("Received echo");},
                Err(e) => {
                    println!("Error:{:?}",e);
                    panic!("Echo response failed");
                },
            }
        });
        
    }).expect("Unable to spin");
    //let cloned = node.clone();

    });

    

    #[service_impl(Example)]
    pub struct EchoServerImpl {}

    impl ServiceInstance for EchoServerImpl {}
    impl ServiceVersion for EchoServerImpl {}

    #[async_trait]
    impl Example for EchoServerImpl {
        
        async fn echo(&self, data: String) -> Result<EchoResponse, ExampleError> {

            let response = EchoResponse {
                echo : data,
            };
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
    let mut node = NodeBuilder::default().build("server".to_owned()).expect("Node creation");

    let server = Arc::new(EchoServerImpl {});

    node.serve(server).expect("Unable to serve");

    node.spin( || {
        println!("Server spinning");

    }).expect("Unable to spin");



   

}
}
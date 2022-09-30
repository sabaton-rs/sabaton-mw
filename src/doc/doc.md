
# <div style="color:red">1.  Creating a sabaton node and publishing a topic from the vehicle-signal crate </div>

This document will help you to create a sabaton Node and publish a sample topic from a vehicle-signal crate.

## <div style="color:blue">1.1 Sabaton Node </div>

Sabaton nodes are applications that interact with the rest of the system using data topics and/or interfaces. Nodes may,

1. Publish data
2. Subscribe to data published by other nodes
3. Host a service
4. Access a services provided by another node  

Nodes will use the functionality of Sabaton Middleware to achieve the above.

### <b> How to create a sabaton node?</b>

#### 1.1.1 Using Default trait implementation for NodeBuilder

 The `NodeBuilder` structure provides a builder pattern to create the node.

```rust  
pub struct NodeBuilder {
    group: String,
    instance: String,
    num_workers: usize,
    single_threaded: bool,
    shared_memory : bool,
    pub_sub_log_level : config::LogLevel,
    rpc_log_level: config::LogLevel,
}
```

We can create a node using the "Default" trait implementation for structure NodeBuilder.
For example:

```rust
let node =    
NodeBuilder::default()   
.build("example-node".to_owned())   
.expect("Node creation error") 
```

The above example, creates a node called "example-node" with default values(mentioned below) for the members of structure "NodeBuilder":  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;group: "default",  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;instance: "0",  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;num_workers: 1,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;single_threaded: true,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;shared_memory : false,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;pub_sub_log_level : 2,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;rpc_log_level: 2,  

If you want to change the default values, different methods are available within the context of the structure `NodeBuilder`. For example, if you want to make "single_threaded" as false, then use the method called `multi_threaded()` as shown below:

```rust
let node =  
 
NodeBuilder::default()  
.multi_threaded() // Enable this if you want a multi-threaded runtime  
.build("example-node".to_owned())   
.expect("Node creation error")  
```

Similarly if you want to change the value of "num_workers" to 2, then you should be using the method called `with_num_workers()` while creating your node as shown below:  

```rust
let node =  
 
NodeBuilder::default()  
.multi_threaded() // Enable this if you want a multi-threaded runtime  
.with_num_workers(2) // Number of work threads. Fixed to 1 for single threaded runtime.    
.build("example-node".to_owned())   
.expect("Node creation error") 
```

You ca explore more on the different methods available for `NodeBuilder` in the following link:
<https://github.com/sabaton-rs/sabaton-mw/blob/61b677ec262b53f52a3e1557775c61228535e2a5/src/lib.rs#L234>

If you are looking for an example implementation for creating a node, please refer to the following link:
<https://github.com/sabaton-rs/diagnostic-manager/blob/bb1d953d0970ac1bbccb3004e3a4292e1b6627dd/src/lib.rs#L22>

#### <b> 1.1.2 Using cargo-generate </b>

Please follow the below mentioned steps to create template for a Sabaton node using cargo-generate:

1. Install cargo-generate :
cargo install cargo-generate
<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/cargo_generate.png" alt="Installing cargo generate;"/>

2. Use cargo generate to create a node:  
cargo generate --git <https://github.com/sabaton-rs/node-template.git>  
<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/Node.png" alt="Node creation;"/>

### <b> 1.2 Pub/Sub Messaging</b>

Publish/subscribe messaging, or pub/sub messaging, is a form of asynchronous service-to-service communication used in serverless and microservices architectures. In a pub/sub model, any message published to a topic is immediately received by all of the subscribers to the topic.

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/Publisher_subscriber.png" alt="Publisher subscriber mechanism;"/>

Vehicle-signal crate generates the DDS Topic types for use in an automotive platform.
Please have a look into the crate before proceeding:
<https://doc.sabaton.dev/public/doc/vehicle_signals/index.html>

You can also have a look into the different possible topics which can be published:
<https://doc.sabaton.dev/public/doc/vehicle_signals/v2/vehicle/index.html>

In a nutshell, to broadcast a message, publisher node simply pushes a message to the topic.All nodes that had subscribed to the topic will receive every message that is broadcast.

### <b> 1.2.1 How to publish a topic?</b>

Follow the below mentioned steps to publish a topic:

1. Use `advertise()` which is a method available within the context of the structure `Node`. This method basically returns a writer for the topic of your choice which can be used to push a message to the chosen topic.
For example, if you want to publish the topic "Speed", then you can use `advertise()` in the following manner:

```rust
let mut SpeedWriter= <name of node>.advertise::<v2::vehicle::Speed>().expect("Unable to advertise");  
```  

2. Use `publish()` on writer returned by `advertise()` to push a message to a given topic as shown below:  

```rust
let speed = Arc::new(Speed::new(KilometrePerHour(10.0), None).unwrap());  //Message to be pushed
let mut res = SpeedWriter.publish(speed.clone());
```

Please refer to the following link to see an example implementation for publishing a topic:
<https://github.com/sabaton-rs/demo_pub/blob/a15df007e6f89f713acc8bbed41b546facf67c83/src/lib.rs#L25>

# <div style="color:red"> 2. Adding a service to an application </div>

Before moving on to the steps to add service to an application, lets brush through the concept(SOME/IP) using which we do the same

## <div style="color:blue"> 2.1 SOME/IP </div>

SOME/IP is a middleware solution that enables service-oriented communication between the control units.

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/SOMEIP.png" alt="SOME/IP.png;"/>

The services are a combination of fields, events, and/or methods. The Server ECU provides a service instance which implements a service interface. The client ECU can use this service instance using SOME/IP to request the required data from the server.

### <b> 2.2 How to add service to an application? </b>

For adding service to an application we use the following crate:

<https://github.com/sjames/someip>

Interfaces are defined by using traits and a derive macro.  
Here is how you would define a service:

```rust
    #[service(
        name("org.hello.service"),
        fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 ;10]value1:Event1, [2;10]value2:String, [3;10]value3: u32), 
        method_ids([2]echo_string, [3]no_reply),
        method_ids([5]echo_struct)
    )]
    #[async_trait]
    pub trait EchoServer {
        fn echo_int(&self, value: i32) -> Result<i32, EchoError>;
        async fn echo_string(&self, value: String) -> Result<String, EchoError>;
        fn no_reply(&self, value: Field1);
        fn echo_u64(&self, value: u64) -> Result<u64, EchoError>;
        fn echo_struct(&self, value : Field1) -> Result<Field1, EchoError>;
    }
```

 Let us use the default node template [(using cargo-generate)](#b-112-using-cargo-generate-b) for our application. We will now try to add a service to the application.

### Steps to be followed

1. Add some-ip ,someip_derive and interface-example crates into your Cargo.toml file:  

```rust
someip = {git = "https://github.com/sjames/someip.git"}
someip_derive = {git = "https://github.com/sjames/someip.git"}

interface-example = { git = "https://github.com/sabaton-rs/interface-example.git"}
```

2. The service which is defined in `interface-example` crate is as shown below:

```rust numberLines
#[service(name("dev.sabaton.ExampleInterface"),
    version(1,0),
    fields([1]status:ExampleStatus)
)]
#[async_trait]
pub trait Example {
    /// Get the list of Software Clusters
    async fn echo(&self, data: String) -> Result<EchoResponse, ExampleError>;   
}

#[derive(Serialize, Deserialize,Clone, Default)]
pub struct EchoResponse  {
    pub echo : String,
}


#[derive(Serialize, Deserialize,Clone)]
pub enum ExampleStatus  {
    Starting,
    Ready,
}

impl Default for ExampleStatus {
    fn default() -> ExampleStatus {
        Self::Starting
    }
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ExampleError {
    #[error("IoError")]
    IoError,
    #[error("Out of Memory")]
    OutOfMemory,
    #[error("Unknown")]
    Unknown,
}
```

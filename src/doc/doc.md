# <div style="color:red">  Creating a sabaton node and publishing a topic from the vehicle-signal crate </div>

This topic will help you to create a sabaton Node and publish a sample topic from vehicle-signal crate.

## <div style="color:blue">1. Sabaton Node </div>

Sabaton nodes are applications that interact with the rest of the system using data topics and/or interfaces. Nodes may,

1. Publish data
2. Subscribe to data published by other nodes
3. Host a service
4. Access a services provided by another node  

Nodes will use the functionality of Sabaton Middleware to achieve the above.

### <b>1.1 How to create a sabaton node?</b>

#### <b>1.1.1 Using Default trait implementation for NodeBuilder</b>

 The `NodeBuilder` structure provides a builder pattern to create the node.


We can create a node using the "Default" trait implementation for structure NodeBuilder.
For example:

```rust
let node =    
NodeBuilder::default()   
.build("example-node".to_owned())   
.expect("Node creation error") 
```

The above example, creates a node called "example-node" with default values for the members of structure "NodeBuilder". 

If you want to change the default values, different methods are available within the context of the structure `NodeBuilder`. For example, if you want to make "single_threaded" as false(default is true), then use the method called `multi_threaded()` as shown below:

```rust
let node =  
 
NodeBuilder::default()  
.multi_threaded() // Enable this if you want a multi-threaded runtime  
.build("example-node".to_owned())   
.expect("Node creation error")  
```

Similarly if you want to change the value of "num_workers" to 2(default is 1), then you should be using the method called `with_num_workers()` while creating your node as shown below:  

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
<https://github.com/sabaton-rs/sabaton-mw/blob/6ee05cf9a54e6267f3b3e9ee1f95ff4d5500c4d3/src/tests.rs#L34>

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

### <b> 1.2.2 How to subscribe to a topic?</b>  

Follow the below mentioned steps to subscribe a topic:  

1. You can use  `subscribe()` or `subscribe_async()` within the context of a node to subscribe to a topic. For instance, if you want to subscribe to a topic called `IsMoving`(in `vehicle-signals` crate),you can refer to the following implementation :

```rust
let mut Moving_reader= node.subscribe_async::<v2::vehicle::IsMoving>().expect("Unable to advertise");
```

2. Create an instance of the topic which you want to subscribe to. For example, you can create an instance of topic `IsMoving` if you want to subscribe to the same:

```rust
let mut moving = Samples::<IsMoving>::new(1);
```

3. Use the reader created in step #1 to read the values of the subscribed topics as shown below:

```rust
 let currentmoving:bool=if let Ok(res) = Moving_reader.take(&mut moving).await
                {   
                    *(moving.get(0).unwrap().value())
                }
                else{
                   true
                };
```

Please refer to the following link to see an example implementation for subscribing a topic: https://github.com/sabaton-rs/demo_sub/blob/be46d6cff83434a5ab1e4d27e80b0b067105b54c/src/lib.rs#L18

# <div style="color:red">2. Creating your own topic library crate </div>

1. Add `cdds_derive` crate into your Cargo.toml file.
2. Import procedural macro called `Topic` in your file:

```rust
   use cdds_derive::Topic;
```

3. Derive `Topic` before defining your topic. For example:

```rust
#[derive(Topic, Deserialize, Serialize, Debug)]
struct SenderType {
    pub msg1: String,
    pub msg2: String,
    pub msg3: Vec<u8>,
    pub inner : Inner,
    pub arr : [String;5],
}
```

As per the above implementation, `SenderType` is the name of the topic.

```rust
let node = NodeBuilder::default()
            .build("testnode".to_string())
            .unwrap();
        let publish_options = PublishOptions::default();
        let mut writer = node.advertise::<SenderType>(&publish_options).unwrap();
```

You can publish or subscribe to the topic as explained in [section 1.2.1 and 1.2.2](#b-121-how-to-publish-a-topicb). For instance:

```rust
let node = NodeBuilder::default()
            .build("testnode".to_string())
            .unwrap();
        let publish_options = PublishOptions::default();
        let mut writer = node.advertise::<SenderType>(&publish_options).unwrap();
```
For better understanding refer to the follwing code :
https://github.com/sabaton-rs/sabaton-mw/blob/main/src/tests.rs

# <div style="color:red"> 3. How to use a service in an application </div>

Before moving on to the steps to add service to an application, lets brush through the concept(SOME/IP) using which we do the same

## <div style="color:blue"> 3.1 SOME/IP </div>

SOME/IP is a middleware solution that enables service-oriented communication between the control units.

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/SOMEIP.png" alt="SOME/IP.png;"/>

The Server ECU provides a service instance which implements a service interface. The client ECU can use this service instance using SOME/IP to request the required data from the server.

## <b> 3.2 Service </b>

Interfaces are defined by using traits (Example in the above example) and a derive macro(service in the above example). The services are a combination of fields, events, and/or methods. A field represents the status of an entity. Event is a message communicated from the server to the client when a value is changed or cyclically communicated to clients. Method is a (programming) function/procedure/subroutine that can be invoked. A method is run on the server on remote invocation from the client.

Let us use the default node template [(using cargo-generate)](#b-112-using-cargo-generate-b) for our application. Default node uses a crate called `interface-example` where a service is already defined for you. The service which is defined in `interface-example` crate is as shown below:

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

In the above example only 1 feild (ExampleStatus) is used. If you want to use more events and methods you can modify your interface definition. For example:

```rust
#[service(name("dev.sabaton.ExampleInterface"),
    version(1,0),
    fields([1]status:ExampleStatus),
     events([1 ;10]value1:String, [2;10]value2:String, [3;10]value3: u32), 
        method_ids([2]echo),
)]
```

But for the time being, let us stick on to the inerface defined in `interface-example`.
We will now try to add the service defined in `interface-example` to an application.

### 3.2.1 <b> How to add service in a Server application?</b>

1. Add some-ip ,someip_derive and interface-example crates into your Cargo.toml file:  

```rust
someip = "0.1.0"
someip_derive = "0.1.0"
interface-example = { git = "https://github.com/sabaton-rs/interface-example.git"}
```

2. Implement the trait(In this case "Example") acording to your requirements in your application. Please find an example implementation of the trait:

```rust
#[service_impl(Example)]
pub struct EchoServerImpl {}

impl ServiceInstance for EchoServerImpl {}
impl ServiceVersion for EchoServerImpl {}

#[async_trait]
    impl Example for EchoServerImpl {
        async fn echo(&self, data: String) -> Result<EchoResponse, ExampleError> {
            println!("Echo is called");
            Ok( EchoResponse {
                echo : data,
            })
        }

        fn set_status(
            &self,
            _status: ExampleStatus,
        ) -> Result<(), someip::error::FieldError> {
            Ok(())
        }

        fn get_status(&self) -> Result<&ExampleStatus, someip::error::FieldError> {
            Ok(&ExampleStatus::Ready)
        } 
    } 
```

As you can see from the above code, `echo()`function of the trait   `Example` is implemented for the structure `EchoServerImpl`. Now your application will act as a server.  A client application can use a service instance to get the required data (fields, events, and methods) from the server. API called `get_status()` can be used by the client for querying `ExampleStatus` and `set_status()` can be used by the client to change/modify `ExampleStatus`.

### <b> 3.2.2 How to utilise a service in a Client application? </b>

Let us again use the default node template [(using cargo-generate)](#b-112-using-cargo-generate-b) for our client application.  

### <b> Steps to be followed </b>

1. Add some-ip ,someip_derive and interface-example crates into your Cargo.toml file:  

```rust
someip = "0.1.0"
someip_derive = "0.1.0"
interface-example = { git = "https://github.com/sabaton-rs/interface-example.git"}
```

2. Default node template creates a default node(in this case `node`) for you (Using `NodeBuilder`). Create a client proxy be using `create_proxy` method as shown below:

```rust
let mut node = NodeBuilder::default()
        .build("example-node".to_owned())
        .expect("Node creation error");

    let client_proxy = node
            .create_proxy::<ExampleProxy>()
            .expect("Unable to create proxy");
```

3. If you want your application to display the reply from server, use the echo() function as shown below:

```rust
let call_properties = CallProperties::with_timeout(Duration::from_millis(15000));

                match client_proxy
                    .echo("Hello".to_string(), &call_properties)
                    .await
                {
                    Ok(res) => {
                        println!("Reply from server: {}", res.echo);
                    }
                    Err(e) => {
                        println!("Error:{:?}", e);
                        panic!("Echo response failed");
                    }
                }

```

An example of a communication between a server(Left side image) and a client(Right side image) is shown below:  

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/server_client.png"   alt="server_client;"/>  

# <div style="color:red"> 4. Creating your own interface library crate </div>

Until now we were using the default interface implementation called `interface-example`. But how to define an interface by your own and use the same in your applications. Please follow the steps below to create your own interface:

1. Clone the default interface implementation into your local repositry using the command :  
   git clone <https://github.com/sabaton-rs/interface-example.git>

2. Modify/add new services as per your requirments.
 Let us now see an example implementation:  

```rust
#[service(name("dev.sabaton.SoftwareUpdate"),
    version(1,0),
    fields([1]status:UpdateStatus),
)]
#[async_trait]
pub trait Example {
    async fn start_update(&self, data: String) -> Result<UpdateStatusResponse, ErrorStatus>;
    
}
#[derive(Serialize, Deserialize,Clone, Default)]
pub struct UpdateStatusResponse  {
    pub echo : String,
}
#[derive(Serialize, Deserialize,Clone)]
pub enum UpdateStatus  {
    Starting,
    Ready,
}

impl Default for UpdateStatus {
    fn default() -> UpdateStatus {
        Self::Starting
    }
}
```

3. Build the interface code.
4. Add the path of the interface into the Cargo.toml file of your application as we have done  in [previous section](#321-b-how-to-add-service-in-a-server-applicationb) and its ready to use.

# <div style="color:red">  Shared memory transport </div>

The shared memory (SHM) transport enables fast communications between entities running in the same processing unit/machine, relying on the shared memory mechanisms provided by the host operating system.

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/shared_memory.png" alt="shared_memory.png;"/>

We can implement the concept of shared memory using iceoryx and cyclonedds.

iceoryx is an inter-process-communication (IPC) middleware for various operating systems.iceoryx uses a true zero-copy, shared memory approach that allows to transfer data from publishers to subscribers without a single copy. This ensures data transmissions with constant latency, regardless of the size of the payload. Following are the steps followed:    
1. A memory pool is created.   
2. Publisher sends a request to pool manager for a shared memory region. 
3. Pool manager gives a handle. 
4. Publisher will put data into the given location and will give handle to subscriber. 
5. Subscriber then uses the handle to access the data. 
6. Finally, subscriber frees the handle which then goes back to the pool.

[SMT.webm](https://user-images.githubusercontent.com/102716966/194551842-291a3217-cebd-4cce-a4fd-6b398f525c5e.webm)

You can have multiple subscribers. Each subscriber gets a handle and can use the memory. After the usage it frees the handle. When all the handles are freed, buffer goes back to pool and can be reused.

Important thing to note here is that memory is allocated by a pool manager and not the publisher. When a publisher wants to publish data, it has to first `loan` a memory region, put data into memory and then publish that memory.

CyclodeDDS checks if iceoryx is available and if publisher and subscriber are on the same machine, it will use the shared memory(using https://github.com/eclipse-iceoryx/iceoryx) concept instead of serializing to a network.

## <b> How to publish a topic? </b>

1. Create a node and enable `shared_memory` as shown below:

```rust
let mut node = NodeBuilder::default().with_shared_memory(true);
```

2.  Define the `PublishOptions` as per your requirements. Please find an example below:
   
```rust
 let mut shm_publish_options = PublishOptions::default();
    let shm_publish_options = shm_publish_options
        .with_durability(sabaton_mw::qos::QosDurability::Volatile)
        .with_reliability(sabaton_mw::qos::QosReliability::Reliable(
            Duration::from_millis(1000),
        ))
        .with_history(sabaton_mw::qos::QosHistory::KeepLast(1));

```

3.  Advertise your topic using `advertise()`, which then returns a writer. For instance, here `Image1080p4BPP` is the topic which is being advertised:    


```rust
 let mut writer = node
        .advertise::<Image1080p4BPP>(&shm_publish_options)
        .unwrap();
```  

4. Loan a memory using the writer which we got in step#3 as shown below:  
   
```rust
 writer.loan();
```

5.  Push your data into the memory.
6.  Initialise the loaned memory as shown below:   

 ```rust
let finalized_image = loaned_image.assume_init();
```

7. Return the loaned buffer as shown below. With that your topic would be published!

 ```rust
writer.return_loan(finalized_image).unwrap();
```

8. Before running the publisher, please run `iox-roudi` which is an iceoryx application by giving the path of config file as a parameter:
 ```rust
 ./iox-roudi -c <CONFIG FILE PATH>
```
You can check for iox-roudi configuration in the following link:  
 https://github.com/sabaton-rs/v4l2-capture-node/blob/main/roudi_config.toml

 <img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/roudi.png" alt="roudi.png;"/>

Please refer to the following link for more details:  
https://github.com/sabaton-rs/v4l2-capture-node/blob/928cd844efdb8672288a9ab86e14bb68232c60f1/src/lib.rs


## <b> How to subscribe to a topic? </b>

1. Create a node and enable `shared_memory` as shown below:

```rust
let mut node = NodeBuilder::default().with_shared_memory(true);
```

2.  Define the `SubscribeOptions` as per your requirements. Please find an example below:
   
```rust
 let mut shm_subscribe_options = SubscribeOptions::default();
```
3. Subscribe to the topic as shown below:
   
```rust
let mut reader= node.subscribe_async::<Image1080p4BPP>(&shm_subscribe_options).expect("Unable to advertise");
```
4. Access the data as explained in [previous-section](#b-122-how-to-subscribe-to-a-topicb)

An example of a communication between a publisher(Left side image) and a subscriber(Right side image) is shown below:

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/SMT_pub_sub.png" alt="SMT_pub_sub.png;"/>

# <div style="color:red"> Creating a sabaton node and publishing a topic from the vehicle-signal crate </div>
This document will help you to create a sabaton Node and publish a sample topic from a vehicle-signal crate.


## <div style="color:blue"> Sabaton Node </div> 
Sabaton nodes are applications that interact with the rest of the system using data topics and/or interfaces. Nodes may,

1. Publish data
2. Subscribe to data published by other nodes
3. Host a service
4. Access a services provided by another node  

Nodes will use the functionality of Sabaton Middleware to achieve the above. 
### <b> How to create a sabaton node?</b>

 The `NodeBuilder` structure provides a builder pattern to create the node.

 pub struct NodeBuilder {  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;group,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;instance,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;num_workers,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;single_threaded,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;shared_memory,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;pub_sub_log_level,  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;rpc_log_level,  
}

We can create a node using the "Default" trait implementation for structure NodeBuilder.
For example:   
let node =   
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; 
NodeBuilder::default()   
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.build("example-node".to_owned())   
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.expect("Node creation error") 

Th above example, creates a node called "example-node" with default values(mentioned below) for the members of structure "NodeBuilder":
    group: "default",
    instance: "0",
    num_workers: 1,
    single_threaded: true,
    shared_memory : false,
    pub_sub_log_level : 2,
    rpc_log_level: 2,

If you want to change the default values, different methods are available within the context of the structure `NodeBuilder`.  The first parameter of a method will be always self, which represents the calling instance of the structure. Methods operate on the data members of a structure. For example, if you want to make "single_threaded" as false, then use the method called `multi_threaded()` as shown below:

let node =  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; 
NodeBuilder::default()  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.multi_threaded() // Enable this if you want a multi-threaded runtime  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.build("example-node".to_owned())   
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.expect("Node creation error")  

Similarly if you want to change the value of "num_workers" to 2, then you should be using the method called `with_num_workers()` while creating your node as shown below:  

let node =  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; 
NodeBuilder::default()  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.multi_threaded() // Enable this if you want a multi-threaded runtime  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.with_num_workers(2) // Number of work threads. Fixed to 1 for single threaded runtime.    
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.build("example-node".to_owned())   
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;.expect("Node creation error") 

You ca explore more on the different methods available for `NodeBuilder` in the following link:
https://github.com/sabaton-rs/sabaton-mw/blob/61b677ec262b53f52a3e1557775c61228535e2a5/src/lib.rs#L234


If you are looking for an example implementation for creating a node, please refer to the following link:
https://github.com/sabaton-rs/diagnostic-manager/blob/bb1d953d0970ac1bbccb3004e3a4292e1b6627dd/src/lib.rs#L22

### <b> Pub/Sub Messaging?</b>

Publish/subscribe messaging, or pub/sub messaging, is a form of asynchronous service-to-service communication used in serverless and microservices architectures. In a pub/sub model, any message published to a topic is immediately received by all of the subscribers to the topic.

<img src="https://github.com/sabaton-rs/sabaton-mw/blob/main/src/doc/Publisher_subscriber.png" alt="Publisher subscriber mechanism" style="height: 300px; width:300px;"/>

Vehicle-signal crate generates the DDS Topic types for use in an automotive platform. 
Please have a look into the crate before proceeding:
https://doc.sabaton.dev/public/doc/vehicle_signals/index.html

You can also have a look into the different possible topics which can be published:
https://doc.sabaton.dev/public/doc/vehicle_signals/v2/vehicle/index.html

In a nutshell, to broadcast a message, publisher node simply pushes a message to the topic.All nodes that had subscribed to the topic will receive every message that is broadcast.

### <b> How to publish a topic?</b>

Follow the below mentioned steps to publish a topic:

1. Use `advertise()` which is a method available within the context of the structure `Node`. This method basically returns a writer for the topic of your choice which can be used to push a message to the chosen topic.
For example, if you want to publish the topic "Speed", then you can use `advertise()` in the following manner:
  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;let mut SpeedWriter= <name of node>.advertise::<v2::vehicle::Speed>().expect("Unable to advertise");  

2. Use `publish()` on writer returned by `advertise()` to push a message to a given topic as shown below:  
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;let speed = Arc::new(Speed::new(KilometrePerHour(10.0), None).unwrap());  //Message to be pushed
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;let mut res = SpeedWriter.publish(speed.clone());

Please refer to the following link to see an example implementation for publishing a topic:
https://github.com/sabaton-rs/demo_pub/blob/a15df007e6f89f713acc8bbed41b546facf67c83/src/lib.rs#L25















